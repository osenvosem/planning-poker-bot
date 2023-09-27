use rusqlite::{params, Connection, Error};
use teloxide::types::{ChatId, MessageId, UserId};

#[derive(Debug)]
pub struct SessionWithInitiator {
    pub session_id: usize,
    pub title: String,
    pub description: String,
    pub finished: u8,
    pub initiator_first_name: String,
    pub initiator_last_name: String,
    pub initiator_username: String,
    pub initiator_db_id: usize,
}

#[derive(Debug)]
pub struct EstimationWithUser {
    pub id: usize,
    pub value: usize,
    pub first_name: String,
    pub last_name: String,
    pub username: String,
}

pub fn prepare_database(conn: &mut Connection) -> Result<(), Error> {
    conn.execute_batch(
        "
            CREATE TABLE IF NOT EXISTS sessions (
                id INTEGER PRIMARY KEY,
                tg_chat_id INTEGER NOT NULL,
                tg_message_id INTEGER NOT NULL,
                title VARCHAR(255),
                description VARCHAR(1000),
                finished TINYINT DEFAULT 0 NOT NULL,
                initiator_id INT NOT NULL,
                FOREIGN KEY(initiator_id) REFERENCES users(id),
                UNIQUE(tg_message_id)
            );

            CREATE TABLE IF NOT EXISTS estimations (
                id INTEGER PRIMARY KEY,
                value INTEGER,
                session_id INT NOT NULL,
                user_id INT NOT NULL,
                FOREIGN KEY(session_id) REFERENCES sessions(id)
                FOREIGN KEY(user_id) REFERENCES users(id)
                UNIQUE(user_id, session_id)
            );

            CREATE TABLE IF NOT EXISTS chat_configs (
                id INTEGER PRIMARY KEY,
                tg_chat_id INT NOT NULL,
                seq VARCHAR(255),
                chat_id INT NOT NULL,
                FOREIGN KEY(chat_id) REFERENCES sessions(chat_id),
                UNIQUE(tg_chat_id)
            );

            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY,
                tg_id INTEGER NOT NULL,
                first_name VARCHAR(64) NOT NULL,
                last_name VARCHAR(64),
                username VARCHAR(32),
                UNIQUE(tg_id)
            );
        
        ",
    )?;

    Ok::<_, Error>(())
}

pub fn save_session(
    conn: &mut Connection,
    tg_chat_id: ChatId,
    tg_message_id: i32,
    title: String,
    description: String,
    initiator_id: String,
) -> Result<usize, Error> {
    conn.execute(
        "
        INSERT INTO sessions (tg_chat_id, tg_message_id, title, description, initiator_id) VALUES (?1, ?2, ?3, ?4, ?5); 
    ",
        [
            tg_chat_id.to_string(),
            tg_message_id.to_string(),
            title,
            description,
            initiator_id,
        ],
    )
}

pub fn find_or_insert_user(
    conn: &mut Connection,
    user_id: UserId,
    first_name: String,
    last_name: String,
    username: String,
) -> Result<usize, Error> {
    let insert_or_update_query = "
        INSERT INTO users (tg_id, first_name, last_name, username)
        VALUES (?1, ?2, ?3, ?4)
        ON CONFLICT(tg_id) DO UPDATE SET
            first_name = ?2,
            last_name = ?3,
            username = ?4
        WHERE tg_id = ?1;
    ";

    let user_id_query = "SELECT id FROM users WHERE tg_id = ?1;";

    let result = conn.execute(
        insert_or_update_query,
        [user_id.to_string(), first_name, last_name, username],
    );

    match result {
        Ok(_) => {
            let mut stmt = conn.prepare(user_id_query)?;
            let mut row = stmt.query_map([user_id.to_string()], |row| Ok(row.get(0)))?;

            row.next().unwrap()?
        }
        Err(error) => Err(error),
    }
}

pub fn find_session_with_initiator(
    conn: &mut Connection,
    chat_id: ChatId,
    message_id: MessageId,
) -> Result<SessionWithInitiator, Error> {
    let query = "
        SELECT sessions.id as session_id, finished, title, description, users.first_name as initiator_first_name, users.last_name as initiator_last_name, users.username as initiator_username, users.id
        FROM sessions
        JOIN users ON sessions.initiator_id = users.id
        WHERE sessions.tg_chat_id = ?1 AND sessions.tg_message_id = ?2;";

    conn.query_row(
        query,
        params![chat_id.to_string(), message_id.to_string()],
        |row| {
            Ok(SessionWithInitiator {
                session_id: row.get(0)?,
                finished: row.get(1)?,
                title: row.get(2)?,
                description: row.get(3)?,
                initiator_first_name: row.get(4)?,
                initiator_last_name: row.get(5)?,
                initiator_username: row.get(6)?,
                initiator_db_id: row.get(7)?,
            })
        },
    )
}

pub fn insert_update_estimation(
    conn: &mut Connection,
    user_id: usize,
    session_id: usize,
    value: String,
) -> Result<usize, Error> {
    let select_query = "
        SELECT COUNT(*)
        FROM estimations
        WHERE session_id = ?1 AND user_id = ?2;
    ";
    let select_result = conn.query_row(
        select_query,
        [session_id.to_string(), user_id.to_string()],
        |row| row.get(0),
    );

    let query = "
        INSERT INTO estimations (value, session_id, user_id)
        VALUES (?1, ?2, ?3)
        ON CONFLICT(user_id, session_id)
        DO UPDATE SET value = ?1
        WHERE session_id = ?2 AND user_id = ?3;
    ";

    let _ = conn.execute(query, [value, session_id.to_string(), user_id.to_string()]);

    select_result
}

pub fn find_estimations(
    conn: &mut Connection,
    session_id: usize,
) -> Result<Vec<EstimationWithUser>, Error> {
    let query = "
        SELECT estimations.id, value, users.first_name, users.last_name, users.username FROM estimations
        JOIN users ON estimations.user_id = users.id
        WHERE estimations.session_id = ?1
    ";

    let mut stmt = conn.prepare(query)?;

    let rows = stmt.query_map([session_id.to_string()], |row| {
        Ok(EstimationWithUser {
            id: row.get(0)?,
            value: row.get::<usize, usize>(1)?,
            first_name: row.get(2)?,
            last_name: row.get(3)?,
            username: row.get(4)?,
        })
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?)
    }

    Ok(result)
}

pub fn restart_session(conn: &mut Connection, session_id: usize) -> Result<usize, Error> {
    let estimations_query = "
        DELETE FROM estimations
        WHERE session_id = ?1;
    ";

    let finish_session_query = "
        UPDATE sessions
        SET finished = 0
        WHERE id = ?1;
    ";

    conn.execute(estimations_query, [session_id.to_string()])?;

    conn.execute(finish_session_query, [session_id.to_string()])
}

pub fn finish_session(conn: &mut Connection, session_id: usize) -> Result<usize, Error> {
    let query = "
        UPDATE sessions
        SET finished = 1
        WHERE id = ?1;
    ";

    conn.execute(query, [session_id.to_string()])
}
