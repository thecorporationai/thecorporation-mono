//! Container configuration builder — hardened sandbox for Pi runtime.
//!
//! Ported from services/agents/agents/sandbox.py.

use std::collections::HashMap;

use bollard::container::{Config, CreateContainerOptions};
use bollard::models::HostConfig;

use crate::config::WorkerConfig;
use crate::domain::agent::AgentDefinition;
use crate::domain::message::InboundMessage;

/// Seccomp profile: deny-list of dangerous syscalls.
/// Default action is ALLOW; we block specific dangerous calls.
const SECCOMP_PROFILE: &str = r#"{
    "defaultAction": "SCMP_ACT_ALLOW",
    "architectures": ["SCMP_ARCH_X86_64", "SCMP_ARCH_AARCH64"],
    "syscalls": [
        {
            "names": [
                "init_module", "finit_module", "delete_module",
                "ptrace",
                "bpf",
                "userfaultfd",
                "unshare", "setns",
                "mount", "umount2", "pivot_root",
                "clock_settime", "clock_adjtime", "settimeofday", "adjtimex",
                "add_key", "request_key", "keyctl",
                "reboot", "kexec_load", "kexec_file_load",
                "swapon", "swapoff",
                "iopl", "ioperm",
                "lookup_dcookie",
                "perf_event_open",
                "open_by_handle_at", "name_to_handle_at",
                "acct",
                "personality",
                "move_pages", "mbind", "set_mempolicy",
                "nfsservctl",
                "vm86", "vm86old"
            ],
            "action": "SCMP_ACT_ERRNO",
            "errnoRet": 1
        }
    ]
}"#;

/// Build a hardened container config for the Pi runtime.
pub fn build_container_config(
    agent_id: &str,
    agent_config_json: &str,
    message_json: &str,
    execution_id: &str,
    agent_def: &AgentDefinition,
    worker_config: &WorkerConfig,
) -> (CreateContainerOptions<String>, Config<String>) {
    let sandbox = &agent_def.sandbox;
    let memory_mb = sandbox.memory_mb.max(worker_config.runtime_memory_mb);
    let cpu_limit = if sandbox.cpu_limit > 0.0 { sandbox.cpu_limit } else { worker_config.runtime_cpu_limit };

    let env = vec![
        format!("AGENT_CONFIG={agent_config_json}"),
        format!("MESSAGE={message_json}"),
        format!("EXECUTION_ID={execution_id}"),
        format!("SECRETS_PROXY_URL={}", worker_config.secrets_proxy_url()),
        "WORKSPACE=/workspace".to_owned(),
        "HOME=/workspace".to_owned(),
        "TMPDIR=/tmp".to_owned(),
        "CORP_AUTO_APPROVE=1".to_owned(),
        "CORP_CONFIG_DIR=/workspace/.corp".to_owned(),
        "PI_CODING_AGENT_DIR=/workspace/.pi/agent".to_owned(),
        format!("CORP_LLM_PROXY_URL={}", worker_config.llm_proxy_url),
    ];

    let workspace_bind = format!(
        "{}/{}:/workspace:rw",
        worker_config.workspace_root, agent_id
    );

    let mut tmpfs = HashMap::new();
    tmpfs.insert("/tmp".to_owned(), format!("size={}m,noexec", sandbox.disk_mb));

    let mut labels = HashMap::new();
    labels.insert("agents.agent_id".to_owned(), agent_id.to_owned());
    labels.insert("agents.execution_id".to_owned(), execution_id.to_owned());
    labels.insert("agents.managed".to_owned(), "true".to_owned());

    let host_config = HostConfig {
        memory: Some((memory_mb as i64) * 1024 * 1024),
        memory_swap: Some((memory_mb as i64) * 1024 * 1024), // no swap
        nano_cpus: Some((cpu_limit * 1e9) as i64),
        pids_limit: Some(256),
        cap_drop: Some(vec!["ALL".to_owned()]),
        security_opt: Some(vec![
            "no-new-privileges:true".to_owned(),
            format!("seccomp={SECCOMP_PROFILE}"),
        ]),
        readonly_rootfs: Some(true),
        binds: Some(vec![workspace_bind]),
        tmpfs: Some(tmpfs),
        extra_hosts: Some(vec!["host.docker.internal:host-gateway".to_owned()]),
        ..Default::default()
    };

    let container_config = Config {
        image: Some(worker_config.runtime_image.clone()),
        env: Some(env),
        host_config: Some(host_config),
        user: Some("65534:65534".to_owned()),
        labels: Some(labels),
        ..Default::default()
    };

    let create_options = CreateContainerOptions {
        name: format!("aw-{}", &execution_id[..12.min(execution_id.len())]),
        platform: None,
    };

    (create_options, container_config)
}

/// Ensure the workspace directory exists on the host.
pub fn ensure_workspace_dir(workspace_root: &str, agent_id: &str) -> std::io::Result<()> {
    let path = format!("{workspace_root}/{agent_id}");
    std::fs::create_dir_all(&path)?;
    Ok(())
}
