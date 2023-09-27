# Scrum poker Telegram bot

Telegram bot for task estimation written in Rust, Teloxide and Sqlite.

https://github.com/osenvosem/planning-poker-bot/assets/6762673/9ee63163-450c-4e7c-9f7d-c1a17a9b345f

Available here: [@devestimbot](https://t.me/devestimbot?startgroup=true).

# Usage

-   Add [@devestimbot](https://t.me/devestimbot?startgroup=true) to the group chat.
-   Use the `/poker [link to task tracker]` command to start an estimation session.
-   Wait for participants to estimate your task then end the session to see the estimations.

# Example

```
/poker https://task.tracker/ISSUE-1234

Optional description.
```

# Features

-   Parses link to task tracker if provided. Can also be task number or any text.
-   Optionally your can add description on a new line.
-   As an initiator you can restart the session or finish it.
