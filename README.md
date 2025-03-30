![Dynamic JSON Badge](https://img.shields.io/badge/dynamic/json?url=https%3A%2F%2Fapi.github.com%2Frepos%2Ftolik518%2Ffactorion-bot%2Fdeployments%3Fper_page%3D1&query=%24.0.ref&label=Deployed%20version&prefix=v) ![GitHub Tag](https://img.shields.io/github/v/tag/tolik518/factorion-bot?label=Current%20version)

<p align="center">
    <img alt="Factorion-Logo which looks like a robot from futurama" src=".github/image_pixelart_transparent.png" width="128px">
</p>

<h1 align="center"> Factorion-bot </h1>

<p align="center"> 
A reddit bot replying to comments, containing factorials, with the solution.  
This little fella is currently running on <b>r/mathmemes</b>, <b>r/unexpectedfactorial</b> and on <b>r/ProgrammerHumor</b>. 
</p>

## Table of Contents

- [Getting Started](#getting-started)
  - [Prerequisites](#prerequisites)
  - [Reddit API Credentials](#reddit-api-credentials)
  - [Installation](#installation)
  - [Configuration](#configuration)
- [Usage](#usage)
- [Contributing](#contributing)

# Getting Started

Follow these steps to set up and run the Factorion bot on your local machine and account.

## Prerequisites
- **Rust** (latest stable version) - [Install Rust](https://www.rust-lang.org/tools/install)
- **Reddit Account** - To run the bot, you'll need a Reddit account. [Reddit](https://www.reddit.com/)
  
## Reddit API Credentials
###### You can go to [Reddit API Documentation](https://www.reddit.com/dev/api) to checkout all the different endpoints you can access. 
1. We need `Application ID` and `Secret Key` so that Reddit can know about our app. [preferences/apps](https://www.reddit.com/prefs/apps)
2. Click the <b>are you a Developer?</b> button to start setting up the bot.

<details>
<summary>Screenshot</summary>
<img src="https://github.com/user-attachments/assets/140056ac-91ce-4178-8703-19451357adce" \>
</details>

3. Fill in the required details:
   - **Name**: Choose a name for your bot.
   - **App type**: Select **Script**.
   - **Redirect URI**: Use `http://localhost:8080` (or any URI; it’s not used for script bots).
     
<details>
<summary>Screenshot</summary>
    <img src="https://github.com/user-attachments/assets/2450994a-14cf-4f46-9f71-518ceb0c59f5" \>
</details>

4. After creating the app, you'll receive:
   - `client_id` (listed under the app name)
   - `client_secret`
   - A `username` and `password` (for the Reddit account that created the app)


### Installation

Fork/Clone the repository and navigate to the project directory:

```bash
git clone https://github.com/yourusername/factorion-bot.git
cd factorion-bot
```

### Configuration

Create a `.env` file in the project root with the following variables:

```env
CLIENT_ID=<your_client_id>
CLIENT_SECRET=<your_client_secret>

USERNAME=<reddit_app_username>
PASSWORD=<reddit_app_password>

SLEEP_BETWEEN_REQUESTS=<sleep_time>
SUBREDDITS=<subreddits>
TERMIAL_SUBREDDITS=<subbreddits_with_termials>
CHECK_MENTIONS=<check_mentions>
CHECK_POSTS=<check_posts>
MENTIONS_EVERY=<check_mentions_every_nth_loop>
POSTS_EVERY=<check_posts_every_nth_loop>

INFLUXDB_HOST=localhost:8889
INFLUXDB_BUCKET=factorion-test
INFLUXDB_TOKEN=<token>
```

Replace with the values you received from the Reddit App creation.
InfluxDB is optional and can be removed if not needed.
The `_EVERY` variables are optional and default to "1".
They control how often posts/mentions are checked compared to comments.
Setting them to "0" will result in a chrash.

## Run the following command to install dependencies:

##### Refer to `Cargo.toml`

```bash
cargo build
```

## Usage

Run the bot with:

```bash
cargo run
```
### How does it work in Reddit?
1. Create a new user for the bot so it can be mentioned by `/u/<botname>`

2. Create a new subreddit `/r/<botname>` as a test play ground.

## Running on a server
The recommended way would be running the bot using docker.

```bash
git clone https://github.com/tolik518/factorion-bot
docker build -t factorion-bot .
# either create a network called `service-network` or remove the network if not needed
docker run --rm -d --name factorion-bot --volume $(pwd):/usr/factorion factorion-bot:latest ./run.sh
```


## Contributing

Feel free to submit issues or pull requests if you would like to contribute to this project.
