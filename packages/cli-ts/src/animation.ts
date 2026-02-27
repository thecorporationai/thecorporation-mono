/**
 * Rising cityscape boot animation — ported from packages/cli/corp/tui/widgets/mascot.py
 */

// prettier-ignore
const BUILDINGS: string[][] = [
  [
    "  ┌┐  ",
    "  ││  ",
    "  ││  ",
    " ┌┤├┐ ",
    " │││││",
    " │││││",
  ],
  [
    " ╔══╗ ",
    " ║▪▪║ ",
    " ║▪▪║ ",
    " ║▪▪║ ",
    " ║▪▪║ ",
    " ║▪▪║ ",
    " ║▪▪║ ",
    " ║▪▪║ ",
  ],
  [
    "  /\\  ",
    " /  \\ ",
    " │▪▪│ ",
    " │▪▪│ ",
    " │▪▪│ ",
  ],
  [
    " ┌──┐ ",
    " │≋≋│ ",
    " │▪▪│ ",
    " │▪▪│ ",
    " │▪▪│ ",
    " │▪▪│ ",
    " │▪▪│ ",
    " │▪▪│ ",
    " │▪▪│ ",
  ],
  [
    "  ╻   ",
    "  ┃   ",
    " ┌┤┐  ",
    " │▪│  ",
    " │▪│  ",
    " │▪│  ",
    " │▪│  ",
    " │▪│  ",
    " │▪│  ",
    " │▪│  ",
    " │▪│  ",
  ],
  [
    " ┌┐  ",
    " ├┤  ",
    " │▪│ ",
    " │▪│ ",
    " │▪│ ",
    " │▪│ ",
  ],
  [
    " ╔═══╗",
    " ║▪ ▪║",
    " ║▪ ▪║",
    " ║▪ ▪║",
    " ║▪ ▪║",
    " ║▪ ▪║",
    " ║▪ ▪║",
    " ║▪ ▪║",
    " ║▪ ▪║",
    " ║▪ ▪║",
  ],
  [
    " ┬─┬ ",
    " │~│ ",
    " │▪│ ",
    " │▪│ ",
    " │▪│ ",
    " │▪│ ",
    " │▪│ ",
  ],
];

const MAX_HEIGHT = Math.max(...BUILDINGS.map((b) => b.length));
const TOTAL_FRAMES = MAX_HEIGHT + 4; // 15 frames, ~1.5s at 100ms

const GOLD = "\x1b[38;2;212;160;23m";
const RESET = "\x1b[0m";

export function renderFrame(frame: number): string {
  const cols: string[][] = [];
  for (let i = 0; i < BUILDINGS.length; i++) {
    const building = BUILDINGS[i];
    const h = building.length;
    const visible = Math.max(0, Math.min(h, frame - i));
    const width = building[0]?.length ?? 6;
    const blank = " ".repeat(width);
    const col: string[] = Array(MAX_HEIGHT - visible).fill(blank);
    col.push(...building.slice(h - visible));
    cols.push(col);
  }

  const lines: string[] = [];
  for (let row = 0; row < MAX_HEIGHT; row++) {
    lines.push(cols.map((col) => col[row]).join(""));
  }
  return lines.join("\n");
}

/**
 * Run an async function while displaying the rising cityscape animation.
 * No-ops when stdout is not a TTY (piped, CI, etc.).
 */
export async function withAnimation<T>(fn: () => Promise<T>): Promise<T> {
  if (!process.stdout.isTTY) {
    return fn();
  }

  let frame = 0;
  let animDone = false;
  const spinChars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
  let spinIdx = 0;
  let lastLineCount = 0;

  const clearPrev = () => {
    if (lastLineCount > 0) {
      process.stdout.write(`\x1b[${lastLineCount}A\x1b[0J`);
    }
  };

  const drawFrame = () => {
    clearPrev();
    if (!animDone) {
      const art = renderFrame(frame);
      const output = `${GOLD}${art}${RESET}\n`;
      process.stdout.write(output);
      lastLineCount = MAX_HEIGHT + 1;
      frame++;
      if (frame >= TOTAL_FRAMES) {
        animDone = true;
      }
    } else {
      // Dot spinner fallback after animation finishes
      const line = `${GOLD}${spinChars[spinIdx % spinChars.length]} Loading...${RESET}\n`;
      process.stdout.write(line);
      lastLineCount = 1;
      spinIdx++;
    }
  };

  drawFrame();
  const timer = setInterval(drawFrame, 100);

  try {
    const result = await fn();
    return result;
  } finally {
    clearInterval(timer);
    clearPrev();
  }
}
