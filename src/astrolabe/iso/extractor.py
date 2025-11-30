"""ISO extraction utilities using 7z."""

from __future__ import annotations

import subprocess
import shutil
from pathlib import Path
from typing import Callable, Optional


class ISOExtractor:
    """Extract files from game ISO using 7z."""

    def __init__(self, iso_path: str | Path):
        """
        Initialize the ISO extractor.

        Args:
            iso_path: Path to the ISO file
        """
        self.iso_path = Path(iso_path)
        if not self.iso_path.exists():
            raise FileNotFoundError(f"ISO file not found: {self.iso_path}")

        # Verify 7z is available
        self._7z_path = shutil.which("7z")
        if not self._7z_path:
            raise RuntimeError("7z not found. Please install p7zip.")

    def __enter__(self) -> "ISOExtractor":
        """Context manager entry."""
        return self

    def __exit__(self, exc_type, exc_val, exc_tb) -> None:
        """Context manager exit."""
        pass

    def list_files(self) -> list[str]:
        """
        List all files in the ISO.

        Returns:
            List of file paths within the ISO
        """
        result = subprocess.run(
            [self._7z_path, "l", "-slt", str(self.iso_path)],
            capture_output=True,
            text=True,
            check=True,
        )

        files = []
        current_path = None
        for line in result.stdout.splitlines():
            if line.startswith("Path = ") and current_path is None:
                # First "Path = " is the archive itself, skip it
                current_path = ""
            elif line.startswith("Path = "):
                files.append(line[7:])

        return files

    def extract_all(
        self,
        output_dir: str | Path,
        progress_callback: Optional[Callable[[str, int, int], None]] = None,
    ) -> Path:
        """
        Extract all files from the ISO.

        Args:
            output_dir: Directory to extract files to
            progress_callback: Optional callback(filename, current, total) for progress

        Returns:
            Path to the extraction directory
        """
        output_path = Path(output_dir)
        output_path.mkdir(parents=True, exist_ok=True)

        # If no progress callback, just extract everything at once
        if progress_callback is None:
            subprocess.run(
                [self._7z_path, "x", "-y", f"-o{output_path}", str(self.iso_path)],
                check=True,
                capture_output=True,
            )
        else:
            # Extract with progress tracking
            files = self.list_files()
            total = len(files)
            for i, filepath in enumerate(files, 1):
                progress_callback(filepath, i, total)
                subprocess.run(
                    [self._7z_path, "x", "-y", f"-o{output_path}", str(self.iso_path), filepath],
                    check=True,
                    capture_output=True,
                )

        return output_path

    def extract_file(self, internal_path: str, output_path: str | Path) -> Path:
        """
        Extract a single file from the ISO.

        Args:
            internal_path: Path within the ISO
            output_path: Where to save the extracted file

        Returns:
            Path to the extracted file
        """
        output = Path(output_path)
        output.parent.mkdir(parents=True, exist_ok=True)

        # Extract to a temp location then move
        temp_dir = output.parent / ".astrolabe_temp"
        temp_dir.mkdir(exist_ok=True)

        try:
            subprocess.run(
                [self._7z_path, "x", "-y", f"-o{temp_dir}", str(self.iso_path), internal_path],
                check=True,
                capture_output=True,
            )

            # Find the extracted file and move it
            extracted = temp_dir / internal_path
            if extracted.exists():
                shutil.move(str(extracted), str(output))
            else:
                raise FileNotFoundError(f"Failed to extract: {internal_path}")
        finally:
            shutil.rmtree(temp_dir, ignore_errors=True)

        return output

    def extract_gamedata(self, output_dir: str | Path) -> Path:
        """
        Extract only the Gamedata folder from the ISO.

        Args:
            output_dir: Directory to extract to

        Returns:
            Path to the extracted Gamedata folder
        """
        output_path = Path(output_dir)
        output_path.mkdir(parents=True, exist_ok=True)

        subprocess.run(
            [
                self._7z_path, "x", "-y",
                f"-o{output_path}",
                str(self.iso_path),
                "Gamedata/*",
            ],
            check=True,
            capture_output=True,
        )

        return output_path / "Gamedata"

    def find_level_files(self) -> dict[str, dict[str, str]]:
        """
        Find all level files in the ISO.

        Returns:
            Dictionary mapping level names to their file paths
        """
        files = self.list_files()
        levels: dict[str, dict[str, str]] = {}

        for f in files:
            f_lower = f.lower()
            if "world/levels/" in f_lower and f_lower.endswith(".sna"):
                # Extract level name from path like "Gamedata/World/Levels/brigand/brigand.sna"
                parts = f.split("/")
                if len(parts) >= 2:
                    level_name = parts[-2]
                    if level_name not in levels:
                        levels[level_name] = {}
                    levels[level_name]["sna"] = f

        # Find associated files for each level
        for f in files:
            f_lower = f.lower()
            for level_name in levels:
                level_dir = f"levels/{level_name}/".lower()
                if level_dir in f_lower:
                    if f_lower.endswith(".rtb"):
                        if "fixlvl" in f_lower:
                            levels[level_name]["fixlvl_rtb"] = f
                        else:
                            levels[level_name]["rtb"] = f
                    elif f_lower.endswith(".rtp"):
                        levels[level_name]["rtp"] = f
                    elif f_lower.endswith(".rtt"):
                        levels[level_name]["rtt"] = f
                    elif f_lower.endswith(".gpt"):
                        levels[level_name]["gpt"] = f
                    elif f_lower.endswith(".ptx"):
                        levels[level_name]["ptx"] = f
                    elif f_lower.endswith(".sda"):
                        levels[level_name]["sda"] = f

        return levels
