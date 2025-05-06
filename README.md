[![GitHub release (latest by date)](https://img.shields.io/github/v/release/ramenhost/dbgmcp)](https://github.com/ramenhost/dbgmcp/releases)
[![Follow @ramenhost](https://img.shields.io/twitter/follow/ramenhost?style=social)](https://twitter.com/ramenhost)

# dbgmcp

A collection of MCP servers to connect various debuggers to LLM agents using Model Context Protocol.
Supported debuggers include:
- GDB (GNU Debugger)
- LLDB (LLVM Debugger)
- PDB (Python Debugger)

## Features
- Separate servers for each debugger. Enable or disable them as needed.
- Load programs into the debugger.
- Execute arbitrary commands in the debugger.
- Supports multiple simultaneous connections.

> [!CAUTION]
> AI agents can execute arbitrary commands inside debuggers, including shell commands. Use at your own risk.

## Installation

Simple way is to pick a pre-built binary for your platform from the [releases page](https://github.com/ramenhost/dbgmcp/releases).
Currently, pre-built binaries are available for the following platforms:
- Linux x86_64 (`x86_64-unknown-linux-musl`)

The binaries are named according to the debugger they support:
- gdb-mcp
- lldb-mcp
- pdb-mcp.

If pre-built binaries are not available for your platform, you can build the project from source.

## Building from source

Requires Rust and Cargo to be installed. You can install them using [rustup](https://www.rust-lang.org/tools/install).

```bash
git clone https://github.com/ramenhost/dbgmcp
cd dbgmcp

cargo build --release
```

This will create MCP server binaries in `target/release/` folder.

</details>

## Usage

### Claude Desktop
1. Open the Claude desktop settings. Click on “Developer” in the left-hand bar of the settings pane, and then click on “Edit Config”. The will create a `claude_desktop_config.json` file and display it in filesystem.
2. Add required MCP servers to the `claude_desktop_config.json`. Below configuration includes all debuggers in Claude (GDB, LLDB and PDB). You can include only the servers you need.
```json
{
  "mcpServers": {
    "gdb": {
      "command": "/path/to/gdb-mcp",
      "args": []
    },
    "lldb": {
      "command": "/path/to/lldb-mcp",
      "args": []
    },
    "pdb": {
      "command": "/path/to/pdb-mcp",
      "args": []
    }
  }
}
```
3. Restart Claude or refresh the page.

For more details, see [Claude docs](https://modelcontextprotocol.io/quickstart/user).

### VS Code Github Copilot

1. Enable MCP support in VS Code settings. To enable MCP support in VS Code, enable the `chat.mcp.enabled` setting.
2. Create a `.vscode/mcp.json` file in your workspace.
3. Add required MCP servers to the `mcp.json` file. Below configuration includes all debuggers in VS Code (GDB, LLDB and PDB). You can include only the servers you need.
```json
{
    "servers": {
        "gdb": {
            "type": "stdio",
            "command": "/path/to/gdb-mcp",
            "args": []
        },
        "lldb": {
            "type": "stdio",
            "command": "/path/to/lldb-mcp",
            "args": []
        },
        "pdb": {
            "type": "stdio",
            "command": "/path/to/pdb-mcp",
            "args": []
        }
    }
}
```
4. Restart VS Code or reload the window.
5. Now, the debugger tools are avilable in Agent mode of Github Copilot chat.

For more details, see [VS Code docs](https://code.visualstudio.com/docs/copilot/chat/mcp-servers).

## Credits
- GDB functionality is rust rewrite of [mcp-gdb](https://github.com/signal-slot/mcp-gdb)
