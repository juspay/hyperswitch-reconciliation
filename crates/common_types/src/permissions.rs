use recon_derive::generate_permissions;

generate_permissions! {
    permissions: [
        ReconIngestion:      { scopes: [Read, Write], entities: [Profile] },
        ReconTransformation: { scopes: [Read, Write], entities: [Profile] },
        ReconException:      { scopes: [Read, Write], entities: [Profile] },
        ReconStagingEntry:   { scopes: [Read, Write], entities: [Profile] },
        ReconTransaction:    { scopes: [Read, Write], entities: [Profile] },
        ReconRule:           { scopes: [Read, Write], entities: [Profile] },
        Account:             { scopes: [Read, Write], entities: [Profile] },
    ]
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::Permission;

    #[test]
    #[allow(clippy::unwrap_used)]
    fn permission_round_trips_as_pascal_case() {
        let p = Permission::ProfileReconIngestionWrite;

        assert_eq!(p.to_string(), "ProfileReconIngestionWrite");
        assert_eq!(
            Permission::from_str("ProfileReconIngestionWrite").unwrap(),
            p
        );
        assert_eq!(
            serde_json::to_string(&p).unwrap(),
            "\"ProfileReconIngestionWrite\""
        );
        assert_eq!(
            serde_json::from_str::<Permission>("\"ProfileReconIngestionWrite\"").unwrap(),
            p
        );
    }
}
