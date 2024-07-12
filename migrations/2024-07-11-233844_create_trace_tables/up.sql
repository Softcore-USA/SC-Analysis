-- Create Trace Sets Table
CREATE TABLE trace_sets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Create Traces Table
CREATE TABLE traces (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    set_id INTEGER NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (set_id) REFERENCES trace_sets(id)
);

-- Create Voltage Readings Table
CREATE TABLE voltage_readings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    trace_id INTEGER NOT NULL,
    timestep FLOAT NOT NULL,
    voltage_value FLOAT NOT NULL,
    FOREIGN KEY (trace_id) REFERENCES traces(id)
);
