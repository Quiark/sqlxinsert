extern crate proc_macro;
use self::proc_macro::TokenStream;

use quote::quote;

use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Fields};

/// 2 -> ( $1,$2 )
fn dollar_values(max: usize) -> String {
    let itr = 1..max + 1;
    itr.into_iter()
        .map(|s| format!("${}", s))
        .collect::<Vec<String>>()
        .join(",")
}

/// Create method for inserting struts into Sqlite database
///
/// ```rust
/// # #[tokio::main]
/// # async fn main() -> anyhow::Result<()>{
/// #[derive(Default, Debug, sqlx::FromRow, sqlxinsert::SqliteInsert)]
/// struct Car {
///     pub car_id: i32,
///     pub car_name: String,
/// }
///
/// let car = Car {
///     car_id: 33,
///     car_name: "Skoda".to_string(),
/// };
///
/// let url = "sqlite::memory:";
/// let pool = sqlx::sqlite::SqlitePoolOptions::new().connect(url).await.unwrap();
///
/// let create_table = "create table cars ( car_id INTEGER PRIMARY KEY, car_name TEXT NOT NULL )";
/// sqlx::query(create_table).execute(&pool).await.expect("Not possible to execute");
///
/// let res = car.insert_raw(&pool, "cars").await.unwrap(); // returning id
/// # Ok(())
/// # }
/// ```
///
#[cfg(feature = "sqlite")]
#[proc_macro_derive(SqliteInsert)]
pub fn derive_from_struct_sqlite(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let fields = match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("expected a struct with named fields"),
    };

    // Attributes -> field names
    let field_name: Vec<_> = fields.iter().map(|field| &field.ident).collect();

    let struct_name = &input.ident;

    let field_length = field_name.len();
    // ( $1, $2)
    let values = dollar_values(field_length);

    let fields_list = quote! {
        #( #field_name ),*
    };
    let columns = format!("{}", fields_list);

    TokenStream::from(quote! {

        impl #struct_name {
            pub fn insert_query(&self, table: &str) -> String
            {
                let sqlquery = format!("insert into {} ( {} ) values ( {} )", table, #columns, #values); //self.values );
                sqlquery
            }

            pub async fn insert_raw(&self, txn: &mut sqlx::Transaction<'_, Sqlite>, table: &str) -> anyhow::Result<sqlx::sqlite::SqliteQueryResult>
            {
                let sql = self.insert_query(table);
                Ok(sqlx::query(&sql)
                #(
                    .bind(&self.#field_name)//         let #field_name: #field_type = Default::default();
                )*
                    .execute(&mut **txn)
                    .await?
                )
            }

            /// Adds placeholders and binds values to the query builder (without parens).
            pub fn sql_add_all_values<'s: 'r, 'r>(&'s self, builder: &'s mut sqlx::query_builder::QueryBuilder<'r, Sqlite>
                ) -> &'s mut sqlx::query_builder::QueryBuilder<'r, Sqlite> 
            {
                let mut sep = builder.separated(", ");
                sep
                #(
                    .push_bind(&self.#field_name)
                )*;
                builder
            }

            /** Adds a list of column names to the query builder (without parens). The order is the
             * same as column values added by `sql_add_all_values`*/
            pub fn sql_add_all_columns<'s, 'r>(&'s self, builder: &'s mut sqlx::query_builder::QueryBuilder<'r, Sqlite>
                ) -> &'s mut sqlx::query_builder::QueryBuilder<'r, Sqlite> {
                let mut sep = builder.separated(", ");
                sep
                #(
                    .push(stringify!(#field_name))
                )*;
                builder
            }
        }
    })
}

/// Generates a method for updating the whole existing object in the database.
/// Requires that the object has a primary key named `id`.
#[cfg(feature = "sqlite")]
#[proc_macro_derive(SqliteUpdate)]
pub fn derive_update_from_struct_sqlite(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let fields = match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("expected a struct with named fields"),
    };

    // Attributes -> field names
    //let field_name = fields.iter().map(|field| &field.ident);
    let field_name2: Vec<_> = fields.iter().map(|field| &field.ident).collect();


    let struct_name = &input.ident;

    //let field_length = field_name.len();
    // ( $1, $2)
    // let values = dollar_values(field_length);

    /*
    let fields_list = quote! {
        #( #field_name ),*
    };
    */
    // let columns = format!("{}", fields_list);
    /*
    let assign_list = quote! {
        #( format!("{} = ?", #field_name3) ),*
    };
    */

    let assigns = field_name2.iter().map(|i| format!(
                "{} = ?", i.as_ref().map_or(String::new(), |it| it.to_string())
    )).collect::<Vec<String>>().join(",");

    TokenStream::from(quote! {

        impl #struct_name {
            pub fn update_query(&self, table: &str) -> String
            {
                let sqlquery = format!("update {} set {} where id = ?", table, #assigns);
                sqlquery
            }

            pub async fn update_raw(&self, txn: &mut sqlx::Transaction<'_, Sqlite>, table: &str, id: &str) -> anyhow::Result<sqlx::sqlite::SqliteQueryResult>
            {
                let sql = self.update_query(table);
                Ok(sqlx::query(&sql)
                #(
                    .bind(&self.#field_name2)
                )*
                    .bind(id)
                    .execute(&mut **txn)
                    .await?)
                
            }

            /** Adds the "column = ?" comma separated list to the query and binds the values
             */
            pub fn sql_add_all_set<'s: 'r, 'r>(&'s self, builder: &'s mut sqlx::query_builder::QueryBuilder<'r, Sqlite>
                ) -> &'s mut sqlx::query_builder::QueryBuilder<Sqlite> {
                let mut sep = builder.separated(", ");
                sep
                #(
                    .push_unseparated(format!("{} = ", stringify!(#field_name2)))
                    .push_bind(&self.#field_name2)
                )*;
                builder
            }
        }
    })
}

/// Create method for inserting struts into Postgres database
///
/// ```rust,ignore
/// # #[tokio::main]
/// # async fn main() -> anyhow::Result<()>{
///
/// #[derive(Default, Debug, std::cmp::PartialEq, sqlx::FromRow)]
/// struct Car {
///     pub id: i32,
///     pub name: String,
/// }
///
/// #[derive(Default, Debug, sqlx::FromRow, sqlxinsert::PgInsert)]
/// struct CreateCar {
///     pub name: String,
///     pub color: Option<String>,
/// }
/// impl CreateCar {
///     pub fn new<T: Into<String>>(name: T) -> Self {
///         CreateCar {
///             name: name.into(),
///             color: None,
///         }
///     }
/// }
/// let url = "postgres://user:pass@localhost:5432/test_db";
/// let pool = sqlx::postgres::PgPoolOptions::new().connect(&url).await.unwrap();
///
/// let car_skoda = CreateCar::new("Skoda");
/// let res: Car = car_skoda.insert::<Car>(pool, "cars").await?;
/// # Ok(())
/// # }
/// ```
///
#[cfg(feature = "postgres")]
#[proc_macro_derive(PgInsert)]
pub fn derive_from_struct_psql(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let fields = match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("expected a struct with named fields"),
    };
    let field_name = fields.iter().map(|field| &field.ident);
    let field_name_values = fields.iter().map(|field| &field.ident);

    let field_length = field_name.len();
    // struct Car { id: i32, name: String }
    // -> ( $1,$2 )
    let values = dollar_values(field_length);

    // struct Car ...
    // -> Car
    let struct_name = &input.ident;

    // struct { id: i32, name: String }
    // -> ( id, name )
    let columns = format!(
        "{}",
        quote! {
            #( #field_name ),*
        }
    );

    TokenStream::from(quote! {
        impl #struct_name {
            fn insert_query(&self, table: &str) -> String
            {
                let sqlquery = format!("insert into {} ( {} ) values ( {} ) returning *", table, #columns, #values); // self.value_list()); //self.values );
                sqlquery
            }

            pub async fn insert<T>(&self, pool: &sqlx::PgPool, table: &str) -> anyhow::Result<T>
            where
                T: Send,
                T: for<'c> sqlx::FromRow<'c, sqlx::postgres::PgRow>,
                T: std::marker::Unpin
            {
                let sql = self.insert_query(table);

                // let mut pool = pool;
                let res: T = sqlx::query_as::<_,T>(&sql)
                #(
                    .bind(&self.#field_name_values)//         let #field_name: #field_type = Default::default();
                )*
                    .fetch_one(pool)
                    .await?;

                Ok(res)
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_test() {
        let itr = 1..4;
        let res = itr
            .into_iter()
            .map(|s| format!("${}", s))
            .collect::<Vec<String>>()
            .join(",");

        assert_eq!(res, "$1,$2,$3");
    }

    #[test]
    fn dollar_value_tes() {
        let res = dollar_values(3);
        assert_eq!(res, "$1,$2,$3");
    }
}
