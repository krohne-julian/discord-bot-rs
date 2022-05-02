# discord-bot-rs

## Introduction
A small personal project of mine to familiarize myself with rust. It aims to help discord server administrators to introduce strict but simple guidelines for channels dedicated to giving and receiving feedback on peoples' work. To achieve this, it utilizes a database that keeps track of open entries and given feedback by users on a per-server basis. Users who give feedback of "sufficient" quality, get a permission to post their own work to get feedback on. If users post without permission, the bot deletes the post.

## Basic mechanics
The bot looks out for posts that are either "feedback requests" or "feedback". Feedback requests are - for the sake of simplicity - defined as a message containing a link or a file. Feedback is defined as a reply to a feedback request. Feedback messages that are sufficiently long (defined in the Config-file) will reward users with a feedback permission to post their own feedback request, after which the permission is used up. Permissions also get revoked if not used within a timespan defined in the Config-file. If the minimum length is not reached, the bot will let the user know as well. The bot also contains a `/open` command to show a list of all users still in need of feedback, including a link to the original message.

## Config file
Server administrators will need to setup a `config.toml` file in the Config-folder for the bot to work properly. A Config-file should contain the following content and structure:
```toml
#The ID of the Bot-Account.
client_id = 111111111111
#An Array of Pairs of Servers (Guilds) and their according Feedback-Channel to listen to. This example only contains one server.
channels = [
    { guild = 1234567890, channel = 0987654321 }
]
#The secret(!) token to login as the bot. Remember to put it in quotes.
discord_token = "S3cr3t_T0k3n_!ns3rted.h3re"
#The minimum message length to classify feedback as sufficient enough to grant a permission.
min_msg_len = 150
#The timespan in which a user can make use of their permission.
permission_timeout_days = 14
```