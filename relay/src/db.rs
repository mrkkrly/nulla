use nulla_core::event::Event;
use nulla_core::message::Filter;
use rusqlite::Connection;

pub struct Db {
    conn: Connection,
}

impl Db {
    pub fn open(path: &str) -> Db {
        let conn = Connection::open(path).expect("open db");
        conn.execute(
            "CREATE TABLE IF NOT EXISTS events (
             id TEXT PRIMARY KEY,
             pubkey TEXT NOT NULL,
             created_at INTEGER NOT NULL,
             kind INTEGER NOT NULL,
             content TEXT NOT NULL,
             sig TEXT NOT NULL
            )",
            (),
        )
        .expect("create table");
        Db { conn }
    }

    pub fn insert(&self, ev: &Event) {
        // INSERT OR IGNORE: same id twice is a no-op (events are immutable)
        self.conn
            .execute(
                "INSERT OR IGNORE INTO events
                  (id, pubkey, created_at, kind, content, sig)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    ev.id,
                    ev.pubkey,
                    ev.created_at as i64,
                    ev.kind,
                    ev.content,
                    ev.sig
                ],
            )
            .expect("insert");
    }

    pub fn query(&self, filter: &Filter) -> Vec<Event> {
        // Prototype approach: pull all, filter in Rust.
        // (SQL-level filtering comes when we extend the query model.)
        let mut stmt = self
            .conn
            .prepare("SELECT id, pubkey, created_at, kind, content, sig FROM events")
            .expect("prepare");
        let rows = stmt
            .query_map((), |row| {
                Ok(Event {
                    id: row.get(0)?,
                    pubkey: row.get(1)?,
                    created_at: row.get::<_, i64>(2)? as u64,
                    kind: row.get(3)?,
                    content: row.get(4)?,
                    sig: row.get(5)?,
                })
            })
            .expect("query");

        rows.filter_map(|r| r.ok())
            .filter(|ev| filter.matches(ev))
            .collect()
    }
}
