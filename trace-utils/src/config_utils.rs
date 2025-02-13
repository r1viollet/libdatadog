// Unless explicitly stated otherwise all files in this repository are licensed under the Apache License Version 2.0.
// This product includes software developed at Datadog (https://www.datadoghq.com/). Copyright 2023-Present Datadog, Inc.

use crate::trace_utils;
use std::env;

pub const PROD_INTAKE_SUBDOMAIN: &str = "trace.agent";

const TRACE_INTAKE_ROUTE: &str = "/api/v0.2/traces";
const TRACE_STATS_INTAKE_ROUTE: &str = "/api/v0.2/stats";

pub fn read_cloud_env() -> Option<(String, trace_utils::EnvironmentType)> {
    if let Ok(res) = env::var("K_SERVICE") {
        // Set by Google Cloud Functions for newer runtimes
        return Some((res, trace_utils::EnvironmentType::CloudFunction));
    }
    if let Ok(res) = env::var("FUNCTION_NAME") {
        // Set by Google Cloud Functions for older runtimes
        return Some((res, trace_utils::EnvironmentType::CloudFunction));
    }
    if let Ok(res) = env::var("WEBSITE_SITE_NAME") {
        // Set by Azure Functions
        return Some((res, trace_utils::EnvironmentType::AzureFunction));
    }
    None
}

pub fn trace_intake_url(site: &str) -> String {
    construct_trace_intake_url(site, TRACE_INTAKE_ROUTE)
}

pub fn trace_intake_url_prefixed(endpoint_prefix: &str) -> String {
    format!("{endpoint_prefix}{TRACE_INTAKE_ROUTE}")
}

pub fn trace_stats_url(site: &str) -> String {
    construct_trace_intake_url(site, TRACE_STATS_INTAKE_ROUTE)
}

pub fn trace_stats_url_prefixed(endpoint_prefix: &str) -> String {
    format!("{endpoint_prefix}{TRACE_STATS_INTAKE_ROUTE}")
}

fn construct_trace_intake_url(prefix: &str, route: &str) -> String {
    format!("https://{PROD_INTAKE_SUBDOMAIN}.{prefix}{route}")
}
