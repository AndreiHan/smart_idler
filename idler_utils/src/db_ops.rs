use anyhow::{Context, Result};
use rusqlite::Connection;
use std::fs;
use std::path::PathBuf;

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
    /// Creates a new instance of `RobotDatabase`.
    ///
    /// # Returns
    ///
    /// Returns `Some(RobotDatabase)` if the connection to the database is successfully established,
    /// otherwise returns `None`.
    #[must_use]
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
    /// Inserts the given `RobotInput` data into the database.
    ///
    /// # Arguments
    ///
    /// * `data` - The `RobotInput` data to be inserted into the database.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the insertion is successful, otherwise returns an `Err` containing the error.
    ///
    /// # Errors
    ///
    /// This function can return an error if there is a problem with the database connection or the insertion operation fails.
    pub fn insert_to_db(&mut self, data: &RobotInput) -> Result<()> {
        match self.connection.execute(
            "INSERT INTO robot (input_time, interval) VALUES (?1, ?2)",
            (&data.input_time, &data.interval),
        ) {
            Ok(_) => {
                info!("Inserted {:?} to db", data);
                self.number_of_items += 1;
                Ok(())
            }
            Err(err) => {
                error!("Failed to insert {:?} to db with err: {}", data, err);
                Err(err.into())
            }
        }
    }
    /// Counts the number of items in the database.
    ///
    /// # Returns
    ///
    /// Returns the number of items in the database.
    ///
    /// # Errors
    ///
    /// This function does not produce any errors.
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
            Ok(()) => debug!("Closed bd conn"),
            Err(err) => error!("Failed to close db conn with err: {:?}", err),
        }
    }
}

fn get_db_connection() -> Result<Connection> {
    let db_path = PathBuf::from(DB_NAME);
    if db_path.is_file() {
        debug!("Found db file: {:?}", &db_path);
        return Connection::open(&db_path).context("Failed to open db");
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
            Err(err.into())
        }
    }
}
