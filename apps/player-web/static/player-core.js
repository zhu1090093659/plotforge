export async function loadManifest(fetchManifest = defaultFetchManifest) {
  return fetchManifest();
}

export async function bootPlayer(options = {}) {
  const root = options.root ?? document;
  const mount = playerRoot(root);
  try {
    const manifest = await loadManifest(options.fetchManifest);
    renderPlayer(manifest, root);
  } catch (error) {
    mount.dataset.state = "error";
    text(root, "status", "Export failed to load");
    text(root, "scene-title", "Unable to load PlotForge export");
    text(root, "beat", error instanceof Error ? error.message : String(error));
  }
}

export function renderPlayer(manifest, root = document) {
  const mount = playerRoot(root);
  const scene =
    manifest.scenes.find((candidate) => candidate.key === manifest.entry_scene) ??
    manifest.scenes[0];
  if (!scene) {
    throw new Error("ExportManifest contains no scenes.");
  }

  const firstBeat = scene.beats[0];
  const image = field(root, "scene-image");
  root.title = manifest.game.title;
  mount.dataset.state = "ready";
  text(root, "status", "Static export");
  text(root, "game-title", manifest.game.title);
  text(root, "scene-title", scene.title);
  text(root, "hook", scene.hook);
  text(root, "beat", firstBeat?.text ?? "");
  image.setAttribute("src", scene.background_asset);
  image.setAttribute("alt", `${scene.title} scene artwork`);

  const choices = field(root, "choices");
  choices.replaceChildren();
  for (const choice of firstBeat?.choices ?? []) {
    const button = root.createElement("button");
    button.type = "button";
    button.className = "pf-choice";
    button.dataset.choiceId = choice.id;
    button.dataset.actionType = choice.action_type;
    button.setAttribute("aria-pressed", "false");
    button.textContent = choice.label;
    button.addEventListener("click", () => {
      for (const existing of choices.querySelectorAll("button")) {
        existing.setAttribute("aria-pressed", "false");
      }
      button.setAttribute("aria-pressed", "true");
      mount.dataset.lastChoice = choice.id;
      text(root, "status", `Selected ${choice.action_type}`);
      text(root, "beat", choice.dramatic_purpose);
    });
    choices.appendChild(button);
  }

  updateViewportMode(root);
  return { sceneKey: scene.key, choiceCount: firstBeat?.choices.length ?? 0 };
}

export function updateViewportMode(root = document) {
  const view = root.defaultView ?? globalThis.window;
  playerRoot(root).dataset.viewport =
    (view?.innerWidth ?? 1024) < 760 ? "mobile" : "desktop";
}

async function defaultFetchManifest() {
  const response = await fetch("./game.json", { cache: "no-store" });
  if (!response.ok) {
    throw new Error(`Unable to load game.json: ${response.status}`);
  }
  return response.json();
}

function playerRoot(root) {
  const mount = root.querySelector("[data-player-root]");
  if (!mount) {
    throw new Error("Missing PlotForge player root.");
  }
  return mount;
}

function field(root, name) {
  const element = root.querySelector(`[data-field="${name}"]`);
  if (!element) {
    throw new Error(`Missing player field: ${name}`);
  }
  return element;
}

function text(root, name, value) {
  field(root, name).textContent = value;
}
