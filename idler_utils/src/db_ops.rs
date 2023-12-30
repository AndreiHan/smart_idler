use std::fs;
use std::path::PathBuf;

use rusqlite::{Connection, Result};

const DB_NAME: &str = "robot.db";

#[derive(Debug)]
pub struct RobotInput {
    pub input_time: String,
    pub interval: String,
}

#[derive(Debug)]
pub struct RobotDatabase {
    connection: Connection,
    pub number_of_items: u32,
}

impl RobotDatabase {
    pub fn new() -> Option<RobotDatabase> {
        let conn = match get_db_connection() {
            Ok(c) => c,
            Err(err) => {
                error!("Failed to establish connection to db with err: {}", err);
                return None;
            }
        };
        let mut db = RobotDatabase {
            connection: conn,
            number_of_items: 0,
        };
        db.count_items_db();
        Some(db)
    }

    pub fn insert_to_db(&mut self, data: &RobotInput) {
        match self.connection.execute(
            "INSERT INTO robot (input_time, interval) VALUES (?1, ?2)",
            (&data.input_time, &data.interval),
        ) {
            Ok(_) => {
                info!("Inserted {:?} to db", data);
                self.number_of_items += 1;
            }
            Err(err) => error!("Failed to insert {:?} to db with err: {}", data, err),
        }
    }

    pub fn count_items_db(&mut self) -> u32 {
        match self
            .connection
            .query_row("select count(*) from robot", [], |r| r.get(0))
        {
            Ok(items) => {
                debug!("Found {:?} bd items", items);
                self.number_of_items = items;
                items
            }
            Err(err) => {
                error!("Failed to count db items with err: {err}");
                0
            }
        }
    }

    pub fn close(self) {
        match self.connection.close() {
            Ok(_) => debug!("Closed bd conn"),
            Err(err) => error!("Failed to close db conn with err: {:?}", err),
        }
    }
}

fn get_db_connection() -> Result<Connection> {
    let db_path = PathBuf::from(DB_NAME);
    if db_path.is_file() {
        debug!("Found db file: {:?}", &db_path);
        return Connection::open(&db_path);
    }

    let _ = fs::File::create(&db_path);
    let conn = Connection::open(&db_path)?;
    match conn.execute(
        "CREATE TABLE robot (
            id INTEGER PRIMARY KEY,
            input_time TEXT NOT NULL,
            interval TEXT NOT NULL
        )",
        (),
    ) {
        Ok(id) => {
            info!("Created table {} with return: {}", DB_NAME, id);
            Ok(conn)
        }
        Err(err) => {
            error!("Failed to create table with err: {}", err);
            Err(err)
        }
    }
}
