// extern crate we're testing, same as any other code will do.
//extern crate gmacro;
use sqlx::{Sqlite};

// #[derive(Default, Debug, sqlx::FromRow)]
#[derive(Default, Debug, sqlx::FromRow, sqlxinsert::SqliteInsert, sqlxinsert::SqliteUpdate)]
struct Car {
    pub id: String,
    pub car_name: String,
    pub passengers: i64,
}

#[tokio::test]
async fn test_macro_sqlite_insert() {
    let car = Car {
        id: String::from("33"),
        car_name: "Skoda".to_string(),
        passengers: 3,
    };

    // bug: https://github.com/launchbadge/sqlx/issues/530
    let url = "sqlite::memory:";

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect(url)
        .await
        .expect("Not possible to create pool");

    let create_table = "create table cars (
        id TEXT PRIMARY KEY,
        car_name TEXT NOT NULL,
        passengers INTEGER
    )";
    sqlx::query(create_table)
        .execute(&pool)
        .await
        .expect("Not possible to execute");

    let mut txn = pool.begin().await.unwrap();

    // ----- insert
    let res = car.insert_raw(&mut txn, "cars").await.unwrap();
    assert_eq!(res.rows_affected(), 1);

    // ----- update
    // println!("SQL:{}", car.update_query("cars"));
    let up = Car {
        id: String::from("33"),
        car_name: "Volkswagen".to_string(),
        passengers: 8,
    };
    let res = up.update_raw(&mut txn, "cars", "33").await.unwrap();
    assert_eq!(res.rows_affected(), 1);
    txn.commit().await.unwrap();

    // ----- check
    let mut txn = pool.begin().await.unwrap();
    let rows = sqlx::query_as::<_, Car>("SELECT * FROM cars")
        .fetch_all(&mut *txn)
        .await
        .expect("Not possible to fetch");
    txn.commit().await.unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, "33");
    assert_eq!(rows[0].car_name, "Volkswagen");
    assert_eq!(rows[0].passengers, 8);

}
