use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use plotforge_schema::ExportManifest;
use plotforge_storage::{StorageError, load_project, write_placeholder_png};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExportError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("json error at {path}: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("static export contains a blocked secret marker: {0}")]
    SecretMarker(String),
}

#[derive(Clone, Debug)]
pub struct ExportReport {
    pub output_dir: PathBuf,
    pub files_written: Vec<PathBuf>,
}

pub fn export_static_web(
    project_path: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
) -> Result<ExportReport, ExportError> {
    let project = load_project(project_path)?;
    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir).map_io(output_dir)?;
    fs::create_dir_all(output_dir.join("assets/generated"))
        .map_io(output_dir.join("assets/generated"))?;

    let assets = project
        .scenes
        .iter()
        .map(|scene| scene.background_asset.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let manifest = ExportManifest {
        game: project.game,
        entry_scene: project.story_state.current_scene_key,
        scenes: project.scenes,
        assets: assets.clone(),
        generated_by: "plotforge-export 0.1.0".into(),
    };
    let data = serde_json::to_string_pretty(&manifest).map_err(|source| ExportError::Json {
        path: output_dir.join("game.json"),
        source,
    })?;
    assert_no_secret_markers(&data)?;

    let index_path = output_dir.join("index.html");
    let data_path = output_dir.join("game.json");
    fs::write(&index_path, player_html()).map_io(&index_path)?;
    fs::write(&data_path, data + "\n").map_io(&data_path)?;

    let mut files_written = vec![index_path, data_path];
    for asset in assets {
        let asset_path = output_dir.join(asset);
        write_placeholder_png(&asset_path)?;
        files_written.push(asset_path);
    }

    Ok(ExportReport {
        output_dir: output_dir.to_path_buf(),
        files_written,
    })
}

fn assert_no_secret_markers(data: &str) -> Result<(), ExportError> {
    for marker in ["OPENAI_API_KEY", "api_key", "secret_key", "sk-"] {
        if data.contains(marker) {
            return Err(ExportError::SecretMarker(marker.into()));
        }
    }
    Ok(())
}

fn player_html() -> &'static str {
    r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>PlotForge Player</title>
    <style>
      body {
        margin: 0;
        font-family: ui-serif, Georgia, serif;
        color: #f6efe2;
        background: #171512;
      }
      main {
        min-height: 100vh;
        display: grid;
        place-items: center;
        padding: 32px;
      }
      article {
        width: min(920px, 100%);
        border: 1px solid #6f5f42;
        background: #242018;
        padding: 24px;
      }
      img {
        width: 100%;
        height: 220px;
        object-fit: cover;
        image-rendering: pixelated;
        background: #51442f;
      }
      button {
        display: block;
        width: 100%;
        margin-top: 10px;
        padding: 12px;
        border: 1px solid #8e7a56;
        background: #362f24;
        color: #f6efe2;
        text-align: left;
      }
    </style>
  </head>
  <body>
    <main>
      <article>
        <h1 id="title">Loading PlotForge export...</h1>
        <img id="scene-image" alt="" />
        <h2 id="scene-title"></h2>
        <p id="hook"></p>
        <p id="beat"></p>
        <div id="choices"></div>
      </article>
    </main>
    <script>
      fetch("./game.json")
        .then((response) => response.json())
        .then((game) => {
          const scene = game.scenes.find((item) => item.key === game.entry_scene) || game.scenes[0];
          document.title = game.game.title;
          document.getElementById("title").textContent = game.game.title;
          document.getElementById("scene-title").textContent = scene.title;
          document.getElementById("scene-image").src = scene.background_asset;
          document.getElementById("hook").textContent = scene.hook;
          document.getElementById("beat").textContent = scene.beats[0]?.text || "";
          const choices = document.getElementById("choices");
          for (const choice of scene.beats[0]?.choices || []) {
            const button = document.createElement("button");
            button.textContent = choice.label;
            button.addEventListener("click", () => {
              document.getElementById("beat").textContent = choice.dramatic_purpose;
            });
            choices.appendChild(button);
          }
        });
    </script>
  </body>
</html>
"#
}

trait IoContext<T> {
    fn map_io(self, path: impl AsRef<Path>) -> Result<T, ExportError>;
}

impl<T> IoContext<T> for Result<T, std::io::Error> {
    fn map_io(self, path: impl AsRef<Path>) -> Result<T, ExportError> {
        self.map_err(|source| ExportError::Io {
            path: path.as_ref().to_path_buf(),
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use plotforge_storage::create_demo_project;

    use super::export_static_web;

    #[test]
    fn exports_static_player_files() {
        let temp = tempfile::tempdir().expect("tempdir");
        let project_path = temp.path().join("project");
        let output_dir = temp.path().join("export");
        create_demo_project(&project_path, false).expect("create demo");

        let report = export_static_web(&project_path, &output_dir).expect("export");

        assert!(output_dir.join("index.html").exists());
        assert!(output_dir.join("game.json").exists());
        assert!(
            output_dir
                .join("assets/generated/court-crisis-001.png")
                .exists()
        );
        assert!(report.files_written.len() >= 3);
    }
}
