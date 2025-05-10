import os
import platform
import shutil
from pathlib import Path
import subprocess

ROOT_DIR = Path(__file__).resolve().parent.parent

def fix_rpath_mac(artifact: Path):
    subprocess.run([
        "install_name_tool",
        "-add_rpath",
        "@loader_path",
        str(artifact),
    ])


def find_aws_lc_fips_sys():
    # Check if building on Windows or macOS
    if platform.system() in ["Windows", "Darwin"]:
        # Set base_dir relative to ROOT_DIR
        base_dir = ROOT_DIR / "target" / "production" / "build"
        target_dir = base_dir.parent

        print(f"build_dir: {base_dir}")
        print(f"target_dir: {target_dir}")
        candidates: list[Path] = []

        # Find directories starting with "aws-lc-fips-sys-"
        for dir_entry in base_dir.iterdir():
            if dir_entry.is_dir() and dir_entry.name.startswith("aws-lc-fips-sys-"):
                if (dir_entry / "output").exists():
                    candidates.append(dir_entry)

        if not candidates:
            raise FileNotFoundError("Failed to find aws-lc-fips-sys candidates")

        # Sort candidates by latest modification time
        candidates.sort(key=lambda p: p.stat().st_mtime, reverse=True)

        # Get the first candidate
        lib_dir = candidates[0]
        artifacts_dir = lib_dir / "out" / "build" / "artifacts"

        if not artifacts_dir.exists():
            raise FileNotFoundError("Failed to find aws-lc-fips-sys artifacts")

        # Find all .dll or .dylib files
        artifacts_candidates = [
            artifact for artifact in artifacts_dir.iterdir()
            if artifact.suffix in [".dll", ".dylib"]
        ]

        # Copy files to the base directory
        for artifact in artifacts_candidates:
            output_area = target_dir / artifact.name
            if output_area.exists():
                print(f"Already copied: {artifact.name}")
                continue

            print(f"Copying: {artifact.name}")
            shutil.copy(artifact, output_area)
        if platform.system() == "Darwin":
            # Fix rpath for macOS
            print("Fixing rpath for macOS")
            fix_rpath_mac(target_dir / "showtimes")

# Example usage
if __name__ == "__main__":
    find_aws_lc_fips_sys()
