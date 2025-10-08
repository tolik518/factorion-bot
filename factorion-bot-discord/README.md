# Factorion Discord Bot

A Discord bot that automatically calculates factorials, subfactorials, and termials in messages.

This bot is part of the [factorion-bot](https://github.com/tolik518/factorion-bot) project and uses the shared `factorion-lib` for mathematical calculations.

## Table of Contents

- [Features](#features)
- [Getting Started](#getting-started)
  - [Prerequisites](#prerequisites)
  - [Discord Bot Setup](#discord-bot-setup)
  - [Installation](#installation)
  - [Configuration](#configuration)
  - [Usage](#usage)
- [Commands](#commands)
- [Examples](#examples)

## Features

- Automatic factorial calculation (e.g., `5!`, `10!`)
- Subfactorial support (e.g., `!5`, `!10`)
- Termial/triangular number calculations (with `[termial]` command)
- Nested factorial expressions (e.g., `(3!)!`)
- Scientific notation for large numbers
- Step-by-step calculation display
- Configurable calculation limits

## Getting Started

### Prerequisites

- **Rust** (latest stable version) - [Install Rust](https://www.rust-lang.org/tools/install)
- **Discord Account** - To create and run the bot
- Build dependencies: openssl, gmp, m4, pkg-config

### Discord Bot Setup

1. Go to the [Discord Developer Portal](https://discord.com/developers/applications)
2. Click "New Application" and give it a name
3. Go to the "Bot" section in the left sidebar
4. Click "Add Bot" and confirm
5. Under the bot's username, click "Reset Token" to get your bot token (save this securely)
6. Enable the following Privileged Gateway Intents:
   - Message Content Intent (required to read message content)
7. Go to OAuth2 > URL Generator
8. Select the following scopes:
   - `bot`
9. Select the following bot permissions:
   - Send Messages
   - Read Message History
   - Read Messages/View Channels
10. Copy the generated URL and open it in your browser to invite the bot to your server

### Installation

#### With Cargo (from workspace)

```bash
cd factorion-bot
cargo build --release -p factorion-bot-discord
# The binary will be in target/release/factorion-bot-discord
```

#### Manually

```bash
cd factorion-bot/factorion-bot-discord
cargo build --release
```

### Configuration

Create a `.env` file in the project root (or in the `factorion-bot-discord` directory) with the following variables:

```env
# Required
DISCORD_TOKEN=<your_discord_bot_token>

# Logging (recommended to reduce verbosity from serenity)
RUST_LOG=info,serenity=warn,tracing=warn

# Optional (with defaults shown)
FLOAT_PRECISION=1000
UPPER_CALCULATION_LIMIT=3000
UPPER_APPROXIMATION_LIMIT=1000000
UPPER_SUBFACTORIAL_LIMIT=100000
UPPER_TERMIAL_LIMIT=100000
UPPER_TERMIAL_APPROXIMATION_LIMIT=1000000
INTEGER_CONSTRUCTION_LIMIT=100000
NUMBER_DECIMALS_SCIENTIFIC=5
```

### Usage

Run the bot with:

```bash
# If built from workspace
./target/release/factorion-bot-discord

# Or with cargo run
cargo run --release -p factorion-bot-discord
```

The bot will connect to Discord and start processing messages in servers where it has been invited.

## Commands

The bot supports inline commands within messages:

- `[short]` or `[shorten]` - Use scientific notation for shorter numbers
- `[steps]` or `[all]` - Show all intermediate calculation steps
- `[termial]` or `[triangle]` - Enable termial/triangular number calculations
- `[no note]` or `[no_note]` - Disable the footer note
- `!short`, `!shorten`, `!steps`, etc. - Alternative command format

You can also use commands to disable features:
- `[long]` - Disable shortening
- `[no steps]` or `[no_steps]` - Hide intermediate steps
- `[no termial]` or `[no_termial]` - Disable termial calculations
- `[note]` - Show the footer note

### Channel Configuration

Server administrators with 'Manage Channels' permission can configure default settings per channel:

- `!factorion config` - Show current channel configuration
- `!factorion config shorten on/off` - Enable/disable default shortening for the channel
- `!factorion config no_note on/off` - Enable/disable default no_note for the channel

These settings apply to all calculations in the channel unless overridden by inline commands in individual messages. Configuration is saved to `channel_config.json` and persists across bot restarts.

## Examples

### Basic Factorial

**Message:**
```
What is 5!?
```

**Bot Reply:**
```
5! = 120
```

### Large Factorial

**Message:**
```
Calculate 100!
```

**Bot Reply:**
```
100! = 9.3326 × 10^157
```

### Nested Factorial

**Message:**
```
What about (3!)!?
```

**Bot Reply:**
```
(3!)! = 720
```

### With Steps

**Message:**
```
[steps] (3!)!
```

**Bot Reply:**
```
3! = 6
(3!)! = 6! = 720
```

### Subfactorial

**Message:**
```
What is !5?
```

**Bot Reply:**
```
!5 = 44
```

### Termial

**Message:**
```
[termial] 10?
```

**Bot Reply:**
```
10? = 55
```

## How It Works

The bot listens to all messages in channels where it has access. When it detects a message containing factorial notation:

1. It parses the message to extract factorial expressions
2. Calculates the results using high-precision arithmetic
3. Formats the output according to the configured limits and commands
4. Sends a reply with the calculated results

The bot will not respond to:
- Its own messages
- Other bots' messages
- Messages it has already processed
- Messages without factorial expressions

## Contributing

See the main [CONTRIBUTING.md](../CONTRIBUTING.md) file in the repository root.

## License

This project is licensed under the MIT License - see the [LICENSE](../LICENSE) file for details.
