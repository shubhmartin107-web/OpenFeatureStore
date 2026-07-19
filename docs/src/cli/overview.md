# CLI Overview

The `ofs` command-line tool provides feature store management from the terminal.

## Installation

```bash
# Via pip after building the Python package
pip install openfeaturestore

# Verify
ofs --help
```

## Global Options

| Option | Description |
|---|---|
| `--project, -p` | Project name (default: "default") |
| `--store, -s` | Store backend (default: "memory") |
| `--help` | Show help message |

## Usage

```bash
ofs [global-options] <command> [command-options]
```
