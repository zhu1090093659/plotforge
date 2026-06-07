use std::{
    env, fs,
    path::{Path, PathBuf},
};

use plotforge_schema::{contract_bundle, validate_contract_bundle};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let mut out_dir = PathBuf::from("contracts");
    let mut check = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--out" => {
                index += 1;
                out_dir = args
                    .get(index)
                    .map(PathBuf::from)
                    .ok_or("--out requires a directory")?;
            }
            "--check" => check = true,
            other => return Err(format!("unsupported argument: {other}").into()),
        }
        index += 1;
    }

    let bundle = contract_bundle();
    validate_contract_bundle(&bundle).map_err(|error| format!("{error:?}"))?;
    let schema = serde_json::to_string_pretty(&bundle.json_schema)? + "\n";
    let typescript = bundle.typescript;

    if check {
        check_file(&out_dir.join("plotforge.schema.json"), &schema)?;
        check_file(&out_dir.join("plotforge.d.ts"), &typescript)?;
        return Ok(());
    }

    fs::create_dir_all(&out_dir)?;
    fs::write(out_dir.join("plotforge.schema.json"), schema)?;
    fs::write(out_dir.join("plotforge.d.ts"), typescript)?;
    Ok(())
}

fn check_file(path: &Path, expected: &str) -> Result<(), Box<dyn std::error::Error>> {
    let actual = fs::read_to_string(path)?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "generated contract drifted: {}; run `cargo run -p plotforge-schema --bin export_contracts -- --out contracts`",
            path.display()
        )
        .into())
    }
}
