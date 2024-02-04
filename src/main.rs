mod constants;
mod db;
mod helpers;

use constants::{DEFAULT_SEQ, FUNC_BUTTONS};
use db::{EstimationWithUser, SessionWithInitiator};
use dotenv::dotenv;
use rand::Rng;
use rusqlite::Result;
use std::env;
use std::error::Error;
use teloxide::{
    prelude::*,
    types::{Chat, InlineKeyboardButton, InlineKeyboardMarkup, Me, MessageId, User},
    utils::{command::BotCommands, markdown},
    RequestError,
};
use tokio_rusqlite::Connection;

#[derive(BotCommands)]
#[command(rename_rule = "lowercase", description = "Команды бота:")]
enum Command {
    #[command(description = "Вывести это сообщение")]
    Help,
    #[command(description = "Начать")]
    Poker(String),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    pretty_env_logger::init();

    let exec_dir = env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .display()
        .to_string();
    let conn = Connection::open(format!("{exec_dir}/poker.db"))
        .await
        .unwrap();

    conn.call(db::prepare_database).await?;

    let bot = Bot::from_env();

    let handler = dptree::entry()
        .branch(
            Update::filter_message()
                .filter_command::<Command>()
                .endpoint(message_handler),
        )
        .branch(Update::filter_callback_query().endpoint(callback_handler));

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![conn])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}

async fn message_handler(
    bot: Bot,
    msg: Message,
    me: Me,
    conn: Connection,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(text) = msg.text() {
        match BotCommands::parse(text, me.username()) {
            Ok(Command::Help) => {
                bot.send_message(msg.chat.id, Command::descriptions().to_string())
                    .await?;
            }
            Ok(Command::Poker(payload)) => {
                if payload.is_empty() {
                    bot.send_message(msg.chat.id, "Отсутствует ссылка или ID задачи.")
                        .await?;
                } else {
                    let User {
                        id: user_id,
                        first_name,
                        last_name,
                        username,
                        ..
                    } = msg.from().unwrap().clone();

                    let (title, description) = helpers::parse_title_and_description(&payload);

                    let user_db_result = conn
                        .call(move |conn| {
                            db::find_or_insert_user(
                                conn,
                                user_id,
                                first_name,
                                last_name.unwrap_or("".to_string()),
                                username.unwrap_or("".to_string()),
                            )
                        })
                        .await;

                    let title_for_db = title.clone();
                    let description_for_db = description.clone();

                    match user_db_result {
                        Ok(db_user_id) => {
                            conn.call(move |conn| {
                                db::save_session(
                                    conn,
                                    msg.chat.id,
                                    msg.id.0 + 1,
                                    title_for_db,
                                    description_for_db,
                                    db_user_id.to_string(),
                                )
                            })
                            .await?;
                        }
                        Err(error) => {
                            panic!("Error user db: {:#?}", error)
                        }
                    };
                    let User {
                        first_name,
                        last_name,
                        username,
                        ..
                    } = msg.from().unwrap().clone();

                    let arg = render_text(
                        &title,
                        &description,
                        &first_name,
                        &last_name.unwrap_or("".to_string()),
                        &username.unwrap_or("".to_string()),
                        0,
                        None,
                    );

                    bot.parse_mode(teloxide::types::ParseMode::MarkdownV2)
                        .send_message(msg.chat.id, arg)
                        .reply_markup(make_keyboard(0, false))
                        .await?;
                }
            }

            Err(_) => {
                bot.send_message(msg.chat.id, "Command not found!").await?;
            }
        }
    }

    Ok(())
}

async fn callback_handler(
    bot: Bot,
    q: CallbackQuery,
    conn: Connection,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    bot.answer_callback_query(q.id).await?;
    let CallbackQuery {
        data: button_value,
        from: user,
        message,
        ..
    } = q;
    let User {
        id: user_id,
        first_name,
        last_name,
        username,
        ..
    } = user;

    let user_db_id = conn
        .call(move |conn| {
            db::find_or_insert_user(
                conn,
                user_id,
                first_name,
                last_name.unwrap_or("".to_string()),
                username.unwrap_or("".to_string()),
            )
        })
        .await;

    let Message {
        chat: Chat { id: chat_id, .. },
        id: message_id,
        ..
    } = message.unwrap();

    let SessionWithInitiator {
        session_id,
        title,
        description,
        finished,
        initiator_first_name,
        initiator_last_name,
        initiator_username,
        initiator_db_id,
        ..
    } = conn
        .call(move |conn| db::find_session_with_initiator(conn, chat_id, message_id))
        .await?;

    let _ = match button_value {
        Some(val) => match val.as_str() {
            "restart" => {
                conn.call(move |conn| db::restart_session(conn, session_id))
                    .await?;

                if initiator_db_id == user_db_id.unwrap() {
                    let _ = send_response_message(
                        bot,
                        chat_id,
                        message_id,
                        &title,
                        &description,
                        &initiator_first_name,
                        &initiator_last_name,
                        &initiator_username,
                        finished,
                        None,
                        true,
                    )
                    .await;
                }

                Ok::<(), rusqlite::Error>(())
            }
            "finish" => {
                conn.call(move |conn| db::finish_session(conn, session_id))
                    .await?;

                let session_id_for_find = session_id.clone();

                let estimations: Vec<EstimationWithUser> = conn
                    .call(move |conn| db::find_estimations(conn, session_id_for_find))
                    .await?;

                if initiator_db_id == user_db_id.unwrap() {
                    let _ = send_response_message(
                        bot,
                        chat_id,
                        message_id,
                        &title,
                        &description,
                        &initiator_first_name,
                        &initiator_last_name,
                        &initiator_username,
                        1,
                        Some(estimations),
                        false,
                    )
                    .await;
                }

                Ok(())
            }
            _ => {
                let session_id_for_insert = session_id.clone();

                let estimation_db_result = conn
                    .call(move |conn| {
                        db::insert_update_estimation(
                            conn,
                            user_db_id.unwrap(),
                            session_id_for_insert,
                            val,
                        )
                    })
                    .await;

                if let Ok(count) = estimation_db_result {
                    if count == 1 {
                        return Ok(());
                    }
                }

                let session_id_for_find = session_id.clone();

                let estimations: Vec<EstimationWithUser> = conn
                    .call(move |conn| db::find_estimations(conn, session_id_for_find))
                    .await?;

                let _ = send_response_message(
                    bot,
                    chat_id,
                    message_id,
                    &title,
                    &description,
                    &initiator_first_name,
                    &initiator_last_name,
                    &initiator_username,
                    finished,
                    Some(estimations),
                    false,
                )
                .await;

                Ok(())
            }
        },
        None => Ok(()),
    };

    Ok(())
}

pub async fn send_response_message(
    bot: Bot,
    chat_id: ChatId,
    message_id: MessageId,
    title: &str,
    description: &str,
    initiator_first_name: &str,
    initiator_last_name: &str,
    initiator_username: &str,
    finished: u8,
    estimations: Option<Vec<EstimationWithUser>>,
    is_restart: bool,
) -> Result<Message, RequestError> {
    bot.parse_mode(teloxide::types::ParseMode::MarkdownV2)
        .edit_message_text(
            chat_id,
            message_id,
            render_text(
                &title,
                &description,
                initiator_first_name,
                initiator_last_name,
                initiator_username,
                finished,
                estimations,
            ),
        )
        .reply_markup(make_keyboard(finished, is_restart))
        .await
}

pub fn render_text(
    title: &str,
    description: &str,
    initiator_first_name: &str,
    initiator_last_name: &str,
    initiator_username: &str,
    session_finished: u8,
    votes: Option<Vec<EstimationWithUser>>,
) -> String {
    let processed_title = if helpers::is_url_valid(&title) {
        if let Some(issue_id) = helpers::extract_issue_id(&title) {
            markdown::link(title, markdown::escape(issue_id.as_str()).as_str())
        } else {
            markdown::escape(format!("{}", title).as_str())
        }
    } else {
        markdown::escape(format!("{}", title).as_str())
    };

    let voted_users_section = if let Some(voters_vec) = votes {
        voters_vec
            .iter()
            .map(|user| {
                let vote_char = if session_finished == 1 {
                    user.value.to_string()
                } else {
                    let idx = rand::thread_rng().gen_range(0..constants::EMOJI_SET.len());
                    constants::EMOJI_SET[idx].to_string()
                };

                format!(
                    "{} - {}\n",
                    vote_char.as_str(),
                    helpers::make_username_line(&user.first_name, &user.last_name, &user.username)
                )
            })
            .collect()
    } else {
        "".to_string()
    };

    format!(
        "Оценка задачи: {}{}\n{}\n{}",
        processed_title,
        if description.to_string().is_empty() {
            "".to_string()
        } else {
            markdown::italic(markdown::escape(format!("\n{}", description).as_str()).as_str())
        },
        markdown::escape(
            format!(
                "\nИнициатор: {}\n",
                helpers::make_username_line(
                    &initiator_first_name.to_string(),
                    &initiator_last_name.to_string(),
                    &initiator_username.to_string(),
                )
                .as_str()
            )
            .as_str()
        ),
        if voted_users_section.is_empty() {
            "".to_string()
        } else {
            format!(
                "Оценки:\n\n{}",
                markdown::escape(voted_users_section.as_str()),
            )
        }
    )
}

fn make_keyboard(finished: u8, is_restart: bool) -> InlineKeyboardMarkup {
    let mut keyboard: Vec<Vec<InlineKeyboardButton>> = Vec::new();

    if finished == 0 || is_restart {
        for items in DEFAULT_SEQ.chunks(4) {
            let row = items
                .iter()
                .map(|item| InlineKeyboardButton::callback(item.to_owned(), item.to_owned()))
                .collect();

            keyboard.push(row);
        }

        keyboard.push(
            FUNC_BUTTONS
                .iter()
                .map(|(label, data)| {
                    InlineKeyboardButton::callback(label.to_owned(), data.to_owned())
                })
                .collect(),
        );
    } else {
        keyboard.push(
            FUNC_BUTTONS
                .iter()
                .filter(|(_, data)| data.to_owned() == "restart")
                .map(|(label, data)| {
                    InlineKeyboardButton::callback(label.to_owned(), data.to_owned())
                })
                .collect(),
        )
    }

    InlineKeyboardMarkup::new(keyboard)
}
