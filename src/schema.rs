// @generated automatically by Diesel CLI.

diesel::table! {
    user (id) {
        id -> Int8,
        username -> Text,
        password -> Text,
    }
}
