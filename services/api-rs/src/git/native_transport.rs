//! Native git smart HTTP protocol handlers.
//!
//! Implements `info/refs`, `upload-pack`, and `receive-pack` directly
//! against the corp_store Valkey backend, without shelling out to git.

use corp_store::git_protocol::{ParsedObject, RefUpdate, RefUpdateResult};
use redis::ConnectionLike;

use super::pack;
use super::pktline;
use super::protocol::{GitService, GitProtocolError};

// ── Info/refs ────────────────────────────────────────────────────────

/// Build the ref advertisement for `GET /info/refs`.
pub fn info_refs(
    con: &mut impl ConnectionLike,
    ws: &str,
    ent: &str,
    service: GitService,
) -> Result<Vec<u8>, GitProtocolError> {
    let branches = corp_store::branch::list_branches(con, ws, ent)
        .map_err(|e| GitProtocolError::SubprocessError(format!("list branches: {e}")))?;

    if branches.is_empty() {
        return Err(GitProtocolError::RepoNotFound);
    }

    let mut buf = Vec::new();

    // Capabilities string.
    let caps = match service {
        GitService::UploadPack => {
            "multi_ack_detailed side-band-64k thin-pack no-progress include-tag"
        }
        GitService::ReceivePack => "report-status delete-refs side-band-64k no-thin",
    };

    // First ref line includes capabilities after NUL byte.
    let mut first = true;
    for branch in &branches {
        let refname = format!("refs/heads/{}", branch.name);
        let line = if first {
            first = false;
            format!("{} {}\0{}\n", branch.head_sha1, refname, caps)
        } else {
            format!("{} {}\n", branch.head_sha1, refname)
        };
        buf.extend_from_slice(&pktline::encode(line.as_bytes()));
    }
    buf.extend_from_slice(pktline::flush());

    Ok(buf)
}

// ── Upload-pack (fetch/clone) ────────────────────────────────────────

/// Handle the `POST /git-upload-pack` request body.
///
/// Parses wants/haves, enumerates needed objects, builds a pack, and
/// returns the full response body (pkt-line + side-band framed).
pub fn upload_pack(
    con: &mut impl ConnectionLike,
    _ws: &str,
    _ent: &str,
    body: &[u8],
) -> Result<Vec<u8>, GitProtocolError> {
    // Parse want/have lines from request body.
    let mut pos = 0;
    let mut wants = Vec::new();
    let mut haves = Vec::new();

    let lines = pktline::read_until_flush(body, &mut pos)
        .map_err(|e| GitProtocolError::SubprocessError(format!("pktline parse: {e}")))?;

    for line in &lines {
        let text = String::from_utf8_lossy(line);
        let text = text.trim();
        if let Some(rest) = text.strip_prefix("want ") {
            // First want line may have capabilities after space.
            let sha = rest.split_whitespace().next().unwrap_or("");
            if sha.len() >= 40 {
                wants.push(sha[..40].to_owned());
            }
        } else if let Some(rest) = text.strip_prefix("have ") {
            let sha = rest.trim();
            if sha.len() >= 40 {
                haves.push(sha[..40].to_owned());
            }
        }
    }

    // There may be more have lines after the flush (multi-round negotiation).
    // For simplicity, also parse post-flush lines.
    while pos < body.len() {
        match pktline::read_pktline(body, &mut pos) {
            Ok(None) => break,
            Ok(Some(line)) => {
                let text = String::from_utf8_lossy(&line);
                let text = text.trim();
                if let Some(rest) = text.strip_prefix("have ") {
                    let sha = rest.trim();
                    if sha.len() >= 40 {
                        haves.push(sha[..40].to_owned());
                    }
                }
                if text == "done" {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    if wants.is_empty() {
        return Err(GitProtocolError::SubprocessError(
            "no want lines in upload-pack request".to_owned(),
        ));
    }

    // Enumerate objects.
    let objects = corp_store::git_protocol::enumerate_objects_for_fetch(con, &wants, &haves)
        .map_err(|e| GitProtocolError::SubprocessError(format!("enumerate objects: {e}")))?;

    // Build pack.
    let pack_data = pack::build_pack(&objects)
        .map_err(|e| GitProtocolError::SubprocessError(format!("build pack: {e}")))?;

    // Build response: NAK + side-band framed pack data.
    let mut response = Vec::new();

    // NAK (no common ancestor found — simplest negotiation).
    response.extend_from_slice(&pktline::encode(b"NAK\n"));

    // Send pack data in side-band-64k chunks (max 65519 bytes per chunk).
    const CHUNK_SIZE: usize = 65519;
    for chunk in pack_data.chunks(CHUNK_SIZE) {
        response.extend_from_slice(&pktline::encode_sideband(pktline::SIDEBAND_DATA, chunk));
    }

    // Flush to signal end of pack.
    response.extend_from_slice(pktline::flush());

    Ok(response)
}

// ── Receive-pack (push) ──────────────────────────────────────────────

/// Handle the `POST /git-receive-pack` request body.
///
/// Parses ref update commands and the pack data, stores objects in
/// corp_store, updates refs and tree state, and returns the status
/// response.
pub fn receive_pack(
    con: &mut impl ConnectionLike,
    ws: &str,
    ent: &str,
    body: &[u8],
) -> Result<Vec<u8>, GitProtocolError> {
    // Parse ref update commands.
    let mut pos = 0;
    let mut ref_updates = Vec::new();

    let lines = pktline::read_until_flush(body, &mut pos)
        .map_err(|e| GitProtocolError::SubprocessError(format!("pktline parse: {e}")))?;

    for line in &lines {
        let text = String::from_utf8_lossy(line);
        let text = text.trim();
        // Format: "old_sha new_sha refname\0capabilities" (first line)
        // or:     "old_sha new_sha refname" (subsequent lines)
        let text = text.split('\0').next().unwrap_or(&text);
        let parts: Vec<&str> = text.split_whitespace().collect();
        if parts.len() >= 3 {
            ref_updates.push(RefUpdate {
                old_sha1: parts[0].to_owned(),
                new_sha1: parts[1].to_owned(),
                refname: parts[2].to_owned(),
            });
        }
    }

    if ref_updates.is_empty() {
        return Err(GitProtocolError::SubprocessError(
            "no ref update commands".to_owned(),
        ));
    }

    // Parse PACK data from remaining body.
    let pack_data = &body[pos..];
    let objects = if pack_data.len() >= 12 && &pack_data[0..4] == b"PACK" {
        let parsed = pack::parse_pack(pack_data)
            .map_err(|e| GitProtocolError::SubprocessError(format!("parse pack: {e}")))?;
        parsed
            .into_iter()
            .map(|o| ParsedObject {
                sha1_hex: o.sha1_hex,
                obj_type: o.obj_type,
                content: o.content,
            })
            .collect::<Vec<_>>()
    } else {
        // No pack data (e.g., delete-only push).
        Vec::new()
    };

    // Process the push.
    let results = corp_store::git_protocol::receive_push(con, ws, ent, &ref_updates, &objects)
        .map_err(|e| GitProtocolError::SubprocessError(format!("receive push: {e}")))?;

    // Build response.
    build_receive_pack_response(&results)
}

fn build_receive_pack_response(
    results: &[RefUpdateResult],
) -> Result<Vec<u8>, GitProtocolError> {
    let mut report = Vec::new();

    // unpack status
    report.extend_from_slice(&pktline::encode(b"unpack ok\n"));

    // Per-ref status.
    for r in results {
        if r.ok {
            report.extend_from_slice(&pktline::encode(
                format!("ok {}\n", r.refname).as_bytes(),
            ));
        } else {
            let reason = r.error.as_deref().unwrap_or("unknown error");
            report.extend_from_slice(&pktline::encode(
                format!("ng {} {}\n", r.refname, reason).as_bytes(),
            ));
        }
    }
    report.extend_from_slice(pktline::flush());

    // Wrap in side-band-64k.
    let mut response = Vec::new();
    response.extend_from_slice(&pktline::encode_sideband(
        pktline::SIDEBAND_DATA,
        &report,
    ));
    response.extend_from_slice(pktline::flush());

    Ok(response)
}
