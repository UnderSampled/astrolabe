"""Command-line interface for Astrolabe."""

from pathlib import Path

import click

from astrolabe.iso import ISOExtractor


@click.group()
@click.version_option()
def main() -> None:
    """Astrolabe: Extract Hype: The Time Quest game data for Godot."""
    pass


@main.command()
@click.argument("iso_path", type=click.Path(exists=True, path_type=Path))
@click.option(
    "--output",
    "-o",
    type=click.Path(path_type=Path),
    default=Path("./extracted"),
    help="Output directory for extracted files.",
)
def extract(iso_path: Path, output: Path) -> None:
    """Extract all files from a game ISO.

    ISO_PATH: Path to the Hype: The Time Quest ISO file.
    """
    click.echo(f"Extracting {iso_path} to {output}")

    def progress(filename: str, current: int, total: int) -> None:
        click.echo(f"[{current}/{total}] {filename}")

    with ISOExtractor(iso_path) as extractor:
        extractor.extract_all(output, progress_callback=progress)

    click.echo(f"Extraction complete! Files written to {output}")


@main.command()
@click.argument("iso_path", type=click.Path(exists=True, path_type=Path))
def list_files(iso_path: Path) -> None:
    """List all files in a game ISO.

    ISO_PATH: Path to the Hype: The Time Quest ISO file.
    """
    with ISOExtractor(iso_path) as extractor:
        for filepath in extractor.list_files():
            click.echo(filepath)


if __name__ == "__main__":
    main()
