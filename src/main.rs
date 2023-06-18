// use std::sync::Arc;
use std::fs::File;
use std::io::{BufRead, BufReader, stdout, Write};
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;
use chrono::{Utc, DateTime};

use dotenv::dotenv;
use tasklist;
use rusqlite::{params, Connection, Result as SqliteResult};

use crossterm::{
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen, Clear},
    cursor::{self, MoveTo},
    style::{self, Print},
    Result as CrosstermResult,
};

use std::sync::mpsc::{self, Sender, RecvTimeoutError};


#[derive(Debug)]
struct LogRecord {
    id: i32,
    timestamp: String,
    event: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>>{
    let (tx, rx): (Sender<()>, _) = mpsc::channel();
    ctrlc::set_handler(move || {
        let _ = tx.send(());
    }).expect("Error setting Ctrl-C handler");

    dotenv().ok(); // This line loads the environment variables from the ".env" file.
    let mut stdout = stdout();

    // Enter alternate screen and hide cursor.
    execute!(stdout, EnterAlternateScreen, cursor::Hide)?;

    loop {
        execute!(stdout, Clear(terminal::ClearType::All))?;        

        // println!("{:?} ", list_all_unique_running_apps());

        let conn = Connection::open("my_database.db")?;
        setup_db(&conn)?;

        // let apps_to_check_for: Vec<String> = vec!["VALORANT.exe".to_string(), "RiotClientServices.exe".to_string()];
        // let running_apps: Vec<String> = check_for_specific_apps(apps_to_check_for);
        let running_apps: Vec<String> = list_all_unique_running_apps();
        // println!("The following apps from the list are running: {:?}", running_apps);


        let now: DateTime<Utc> = Utc::now();
        let timestamp_str = now.format("%Y-%m-%d %H:%M:%S").to_string() ;  
        insert_into_db(&conn, timestamp_str.clone(), running_apps.clone())?;

        // Query the log records
        // print_from_db(&conn)?;
        execute!(
            stdout,
            MoveTo(1, 1),
            Print("Last Updated: ".to_string() + &timestamp_str)
        )?;

        // Print "Last Logged Apps" at (1, 2).
        execute!(
            stdout,
            MoveTo(1, 2),
            Print("Last Logged Apps: [".to_string() + &running_apps.join(", ") + "]")
        )?;
        match rx.recv_timeout(Duration::from_secs(10)) {
            Ok(_) | Err(RecvTimeoutError::Disconnected) => {
                break;
            },
            Err(RecvTimeoutError::Timeout) => {},
        }
    }
    execute!(stdout, LeaveAlternateScreen, cursor::Show)?;

    Ok(())
}

fn list_all_unique_running_apps() -> Vec<String> {
    let apps_to_ignore = read_lines("ignored_apps.txt");
    // Lists all unique running apps
    let mut running_apps: Vec<String> = Vec::new();
    unsafe{
        let tl = tasklist::Tasklist::new();
        for i in tl{
            if !running_apps.contains(&i.get_pname()) && !apps_to_ignore.contains(&i.get_pname()){
                running_apps.push(i.get_pname());
            }
        }
    }
    return running_apps;
}

fn read_lines<P>(filename: P) -> Vec<String>
where P: AsRef<Path>, {
    let file = File::open(filename).expect("Could not open file");
    let reader = BufReader::new(file);
    let lines = reader.lines().map(|line| line.expect("Could not read line")).collect();
    lines
}

fn setup_db(conn: &Connection) -> Result<(), Box<dyn std::error::Error>>{
    // Create a table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS running_apps (
            id              INTEGER PRIMARY KEY,
            timestamp       TEXT NOT NULL,
            event           TEXT NOT NULL
        )",
        [],
    )?;

    Ok(())
}

fn insert_into_db(conn: &Connection, timestamp: String, events: Vec<String>) -> Result<(), Box<dyn std::error::Error>>{
    // Insert data
    for event in events{
        conn.execute(
            "INSERT INTO running_apps (timestamp, event) VALUES (?1, ?2)",
            params![timestamp, event],
        )?;
    }

    Ok(())
}

fn print_from_db(conn: &Connection) -> Result<(), Box<dyn std::error::Error>>{
    // Query the log records
    let mut stmt = conn.prepare("SELECT id, timestamp, event FROM running_apps")?;
    let log_iter = stmt.query_map([], |row| {
        Ok(LogRecord {
            id: row.get(0)?,
            timestamp: row.get(1)?,
            event: row.get(2)?,
        })
    })?;

    // Iterate through the log records
    for log in log_iter {
        println!("{:?}", log?);
    }

    Ok(())
}

fn check_for_specific_apps(apps: Vec<String>) -> Vec<String>{
    // Checks existing apps against the list of apps to check for and returns a list of apps that are running
    let mut running_apps: Vec<String> = Vec::new();
    unsafe{
        let tl = tasklist::Tasklist::new();
        for i in tl{
            if apps.contains(&i.get_pname()){
                running_apps.push(i.get_pname());
            }
        }
    }
    return running_apps;
}

