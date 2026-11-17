ALTER TABLE entry
    ALTER COLUMN linking_status DROP NOT NULL,
    ALTER COLUMN linking_data DROP NOT NULL;
