// Unless explicitly stated otherwise all files in this repository are licensed under the Apache License Version 2.0.
// This product includes software developed at Datadog (https://www.datadoghq.com/). Copyright 2021-Present Datadog, Inc.

use datadog_profiling::api;
use datadog_profiling::internal::Profile;
use std::io::Write;
use std::process::exit;
use std::time::SystemTime;

// Keep this in-sync with profiles.c
fn main() {
    let walltime = api::ValueType {
        r#type: "wall-time",
        unit: "nanoseconds",
    };
    let sample_types = vec![
        api::ValueType {
            r#type: "samples",
            unit: "count",
        },
        walltime,
    ];

    let period = api::Period {
        r#type: walltime,
        value: 10000,
    };

    let mapping = api::Mapping {
        filename: "/usr/local/bin/php",
        ..Default::default()
    };
    let sample = api::Sample {
        locations: vec![
            api::Location {
                mapping,
                function: api::Function {
                    name: "phpinfo",
                    filename: "/srv/public/index.php",
                    ..Default::default()
                },
                line: 3,
                ..Default::default()
            },
            api::Location {
                mapping,
                function: api::Function {
                    name: "{main}",
                    filename: "/srv/public/index.php",
                    ..Default::default()
                },
                ..Default::default()
            },
        ],
        values: vec![1, 10000],
        labels: vec![],
    };

    // Intentionally use the current time.
    let mut profile = Profile::new(SystemTime::now(), &sample_types, Some(period));

    match profile.add_sample(sample, None) {
        Ok(_) => {}
        Err(_) => exit(1),
    }

    match profile.serialize_into_compressed_pprof(None, None) {
        Ok(encoded_profile) => {
            let buffer = &encoded_profile.buffer;
            assert!(buffer.len() > 100);
            std::io::stdout()
                .write_all(buffer.as_slice())
                .expect("write to succeed");
        }
        Err(_) => exit(1),
    }
}
