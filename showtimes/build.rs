use std::{path::PathBuf, process::Command};

fn find_aws_lc_fips_sys() {
    // Check if building on Windows or macOS
    if cfg!(target_os = "windows") || cfg!(target_os = "macos") {
        // If yes, let's go find the .dll/.dylib
        let out_dir = std::env::var("OUT_DIR").unwrap();

        // Go the the current_dir/target/PROFILE/build/aws-lc-fips-sys-{HASH}
        // get this file directory
        let build_dir = PathBuf::from(&out_dir).join("..").join("..");
        let base_dir = build_dir.join("..");

        // read dir for aws-lc-fips-sys prefix
        println!("build_dir: {:?}", build_dir);
        let directories = std::fs::read_dir(&build_dir).unwrap();
        let mut candidates = vec![];
        for dir in directories {
            let dir = dir.unwrap();
            let filename = dir.file_name();
            let file_name_str = filename.to_string_lossy();
            if file_name_str.starts_with("aws-lc-fips-sys-") {
                // check if dir has output file
                let output_file = dir.path().join("output").exists();
                if !output_file {
                    continue;
                }

                candidates.push(dir.path());
            }
        }

        // Check which one has the latest mtime
        if candidates.is_empty() {
            panic!("Failed to find aws-lc-fips-sys library");
        }

        candidates.sort_by(|a, b| {
            let a_mtime = a.metadata().unwrap().modified().unwrap();
            let b_mtime = b.metadata().unwrap().modified().unwrap();

            // Latest mtime
            b_mtime.cmp(&a_mtime)
        });

        // get first one
        let lib_dir = candidates.first().unwrap();
        let artifacts = lib_dir.join("out").join("build").join("artifacts");

        if !artifacts.exists() {
            panic!("Failed to find aws-lc-fips-sys library");
        }

        // Get all dll/dylib, exp/lib
        let artifacts = std::fs::read_dir(&artifacts).unwrap();

        let mut artifacts_candidates = vec![];

        for artifact in artifacts {
            let artifact = artifact.unwrap();
            let file_type = artifact.file_name();
            let file_name_str = file_type.to_string_lossy();

            if file_name_str.ends_with(".dll") || file_name_str.ends_with(".dylib") {
                artifacts_candidates.push(artifact.path());
            }
        }

        // Make a copy to the base dir
        for artifact in artifacts_candidates {
            let artifact_name = artifact.file_name().unwrap().to_string_lossy();
            let output_area = base_dir.join(artifact_name.to_string());

            if output_area.exists() {
                // Already copied
                println!("Already copied: {}", artifact_name);
                continue;
            }

            println!("Copying: {}", artifact_name);
            std::fs::copy(&artifact, &output_area).unwrap();
        }
    }
}

fn main() {
    let process = Command::new("git").args(["rev-parse", "HEAD"]).output();

    match process {
        Ok(output) => {
            let commit = String::from_utf8_lossy(&output.stdout);

            println!("cargo:rustc-env=GIT_COMMIT={}", commit);
        }
        Err(_) => {
            println!("cargo:rustc-env=GIT_COMMIT=unknown");
        }
    }

    find_aws_lc_fips_sys();
}
