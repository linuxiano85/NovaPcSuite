"""CLI interface for NovaPcSuite."""

import click
from rich.console import Console
from rich.table import Table

from nova.adb.client import ADBClient
from nova.adb.device import get_device_info


console = Console()


@click.group()
@click.version_option(version="0.1.0", prog_name="novapcsuite")
def cli() -> None:
    """NovaPcSuite - Android device backup and restore suite for Linux."""
    pass


@cli.group()
def device() -> None:
    """Device management commands."""
    pass


@device.command()
def info() -> None:
    """Show device information."""
    try:
        adb_client = ADBClient()
        devices = adb_client.list_devices()
        
        if not devices:
            console.print("[red]No devices found. Make sure a device is connected and USB debugging is enabled.[/red]")
            return
        
        if len(devices) > 1:
            console.print("[yellow]Multiple devices found. Using the first one.[/yellow]")
        
        device_id = devices[0]
        device_info = get_device_info(adb_client, device_id)
        
        table = Table(title="Device Information")
        table.add_column("Property", style="cyan", no_wrap=True)
        table.add_column("Value", style="magenta")
        
        for key, value in device_info.items():
            table.add_row(key, str(value))
        
        console.print(table)
        
    except Exception as e:
        console.print(f"[red]Error getting device info: {e}[/red]")


@cli.group()
def backup() -> None:
    """Backup management commands."""
    pass


@backup.command()
@click.option("--output", "-o", default="./backups", help="Output directory for backups")
def create(output: str) -> None:
    """Create a new backup."""
    console.print("[green]Backup functionality coming soon![/green]")
    console.print(f"Would backup to: {output}")


def main() -> None:
    """Main entry point."""
    cli()


if __name__ == "__main__":
    main()