"""ISO extraction functionality using pycdlib with 7z fallback."""

import shutil
import subprocess
from pathlib import Path
from typing import Callable

import pycdlib


class ISOExtractor:
    """Extract files from a game ISO image.

    Uses pycdlib for standard ISOs, falls back to 7z for non-standard ones
    (like Hype: The Time Quest which has multiple Primary Volume Descriptors).
    """

    def __init__(self, iso_path: Path) -> None:
        """Initialize the extractor with a path to an ISO file.

        Args:
            iso_path: Path to the ISO file to extract from.
        """
        self.iso_path = iso_path
        self._iso: pycdlib.PyCdlib | None = None
        self._use_7z = False

    def __enter__(self) -> "ISOExtractor":
        """Open the ISO file for reading."""
        try:
            self._iso = pycdlib.PyCdlib()
            self._iso.open(str(self.iso_path))
            self._use_7z = False
        except pycdlib.pycdlibexception.PyCdlibInvalidISO:
            # Fall back to 7z for non-standard ISOs
            if shutil.which("7z") is None:
                raise RuntimeError(
                    "ISO has non-standard format and 7z is not installed. "
                    "Please install p7zip or 7zip."
                )
            self._use_7z = True
            self._iso = None
        return self

    def __exit__(self, exc_type: object, exc_val: object, exc_tb: object) -> None:
        """Close the ISO file."""
        if self._iso is not None:
            self._iso.close()
            self._iso = None

    def list_files(self) -> list[str]:
        """List all files in the ISO.

        Returns:
            List of file paths within the ISO.
        """
        if self._use_7z:
            return self._list_files_7z()

        if self._iso is None:
            raise RuntimeError("ISO not opened. Use 'with' statement.")

        files: list[str] = []
        for dirname, _, filenames in self._iso.walk(iso_path="/"):
            for filename in filenames:
                # Remove version suffix (;1) from ISO9660 filenames
                clean_name = filename.rsplit(";", 1)[0]
                if dirname == "/":
                    files.append(f"/{clean_name}")
                else:
                    files.append(f"{dirname}/{clean_name}")
        return files

    def _list_files_7z(self) -> list[str]:
        """List files using 7z."""
        result = subprocess.run(
            ["7z", "l", "-slt", str(self.iso_path)],
            capture_output=True,
            text=True,
            check=True,
        )
        files: list[str] = []
        current_path: str | None = None
        is_dir = False

        for line in result.stdout.splitlines():
            if line.startswith("Path = "):
                current_path = line[7:]
            elif line.startswith("Attributes = "):
                is_dir = "D" in line[13:]
            elif line == "" and current_path is not None:
                # End of entry
                if not is_dir and current_path != str(self.iso_path):
                    files.append("/" + current_path)
                current_path = None
                is_dir = False

        return files

    def extract_all(
        self,
        output_dir: Path,
        progress_callback: Callable[[str, int, int], None] | None = None,
    ) -> None:
        """Extract all files from the ISO to the output directory.

        Args:
            output_dir: Directory to extract files to.
            progress_callback: Optional callback(filename, current, total) for progress.
        """
        if self._use_7z:
            self._extract_all_7z(output_dir, progress_callback)
            return

        if self._iso is None:
            raise RuntimeError("ISO not opened. Use 'with' statement.")

        files = self.list_files()
        total = len(files)

        for i, filepath in enumerate(files):
            if progress_callback:
                progress_callback(filepath, i + 1, total)

            # Create output path
            rel_path = filepath.lstrip("/")
            out_path = output_dir / rel_path
            out_path.parent.mkdir(parents=True, exist_ok=True)

            # Extract file
            iso_path = filepath + ";1"  # Add back version suffix for pycdlib
            self._iso.get_file_from_iso(str(out_path), iso_path=iso_path)

    def _extract_all_7z(
        self,
        output_dir: Path,
        progress_callback: Callable[[str, int, int], None] | None = None,
    ) -> None:
        """Extract all files using 7z."""
        output_dir.mkdir(parents=True, exist_ok=True)

        if progress_callback:
            progress_callback("Extracting with 7z...", 0, 1)

        subprocess.run(
            ["7z", "x", "-y", f"-o{output_dir}", str(self.iso_path)],
            check=True,
            capture_output=True,
        )

        if progress_callback:
            progress_callback("Extraction complete", 1, 1)

    def extract_file(self, iso_filepath: str, output_path: Path) -> None:
        """Extract a single file from the ISO.

        Args:
            iso_filepath: Path to file within the ISO (e.g., "/GAMEDATA/LEVELS.DAT").
            output_path: Local path to write the extracted file.
        """
        if self._use_7z:
            self._extract_file_7z(iso_filepath, output_path)
            return

        if self._iso is None:
            raise RuntimeError("ISO not opened. Use 'with' statement.")

        output_path.parent.mkdir(parents=True, exist_ok=True)
        iso_path = iso_filepath + ";1"
        self._iso.get_file_from_iso(str(output_path), iso_path=iso_path)

    def _extract_file_7z(self, iso_filepath: str, output_path: Path) -> None:
        """Extract a single file using 7z."""
        import tempfile

        # 7z doesn't support extracting to a specific path directly,
        # so we extract to temp and move
        rel_path = iso_filepath.lstrip("/")
        with tempfile.TemporaryDirectory() as tmpdir:
            subprocess.run(
                ["7z", "x", "-y", f"-o{tmpdir}", str(self.iso_path), rel_path],
                check=True,
                capture_output=True,
            )
            extracted = Path(tmpdir) / rel_path
            if extracted.exists():
                output_path.parent.mkdir(parents=True, exist_ok=True)
                shutil.move(str(extracted), str(output_path))
