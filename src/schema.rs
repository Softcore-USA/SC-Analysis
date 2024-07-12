// @generated automatically by Diesel CLI.

diesel::table! {
    trace_sets (id) {
        id -> Nullable<Integer>,
        created_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    traces (id) {
        id -> Nullable<Integer>,
        set_id -> Integer,
        created_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    voltage_readings (id) {
        id -> Nullable<Integer>,
        trace_id -> Integer,
        timestep -> Float,
        voltage_value -> Float,
    }
}

diesel::joinable!(traces -> trace_sets (set_id));
diesel::joinable!(voltage_readings -> traces (trace_id));

diesel::allow_tables_to_appear_in_same_query!(
    trace_sets,
    traces,
    voltage_readings,
);
