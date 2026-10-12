/** Replaces the web view menu. Editing keeps Cut, Copy, Paste, and Select All. */

let menu: HTMLDivElement | null = null;

function closeMenu(): void {
  menu?.remove();
  menu = null;
}

function editableFrom(target: EventTarget | null): HTMLElement | null {
  const element = target instanceof Element ? target : target instanceof Node ? target.parentElement : null;
  const found = element?.closest("input, textarea, [contenteditable='true']");
  if (!(found instanceof HTMLElement)) {
    return null;
  }
  if (found instanceof HTMLInputElement || found instanceof HTMLTextAreaElement) {
    if (found.disabled || found.readOnly) {
      return null;
    }
  }
  return found;
}

function runEdit(field: HTMLElement, command: "cut" | "copy" | "paste" | "selectAll"): void {
  field.focus();
  if (
    command === "selectAll" &&
    (field instanceof HTMLInputElement || field instanceof HTMLTextAreaElement)
  ) {
    field.select();
    return;
  }
  document.execCommand(command);
}

function showMenu(x: number, y: number, field: HTMLElement): void {
  closeMenu();
  const panel = document.createElement("div");
  panel.className = "edit-menu";
  panel.setAttribute("role", "menu");
  for (const item of [
    ["Cut", "cut"],
    ["Copy", "copy"],
    ["Paste", "paste"],
    ["Select All", "selectAll"],
  ] as const) {
    const button = document.createElement("button");
    button.type = "button";
    button.textContent = item[0];
    button.addEventListener("click", () => {
      runEdit(field, item[1]);
      closeMenu();
    });
    panel.append(button);
  }
  panel.style.left = `${x}px`;
  panel.style.top = `${y}px`;
  document.body.append(panel);
  const bounds = panel.getBoundingClientRect();
  if (bounds.right > window.innerWidth) {
    panel.style.left = `${Math.max(0, x - bounds.width)}px`;
  }
  if (bounds.bottom > window.innerHeight) {
    panel.style.top = `${Math.max(0, y - bounds.height)}px`;
  }
  menu = panel;
}

export function installContextMenu(): void {
  document.addEventListener(
    "contextmenu",
    (event) => {
      event.preventDefault();
      const field = editableFrom(event.target);
      if (!field) {
        closeMenu();
        return;
      }
      showMenu(event.clientX, event.clientY, field);
    },
    true,
  );
  window.addEventListener(
    "pointerdown",
    (event) => {
      if (!menu || (event.target instanceof Node && menu.contains(event.target))) {
        return;
      }
      closeMenu();
    },
    true,
  );
  window.addEventListener("keydown", (event) => {
    if (event.key === "Escape") {
      closeMenu();
    }
  });
  window.addEventListener("blur", closeMenu);
  window.addEventListener("scroll", closeMenu, true);
}
