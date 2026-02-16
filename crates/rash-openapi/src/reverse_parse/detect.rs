use regex::Regex;

/// Detected framework from source code analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectedFramework {
    Express,
    Fastify,
    Hono,
    Unknown,
}

/// Detect the web framework used in the given source code.
///
/// Checks import/require patterns to identify the framework.
pub fn detect_framework(source: &str) -> DetectedFramework {
    // Express patterns
    let express_re =
        Regex::new(r#"(?:import\s+express|require\s*\(\s*["']express["']\s*\))"#).unwrap();
    if express_re.is_match(source) {
        return DetectedFramework::Express;
    }

    // Fastify patterns
    let fastify_re =
        Regex::new(r#"(?:import\s+Fastify|import\s+fastify|require\s*\(\s*["']fastify["']\s*\))"#)
            .unwrap();
    if fastify_re.is_match(source) {
        return DetectedFramework::Fastify;
    }

    // Hono patterns
    let hono_re =
        Regex::new(r#"(?:import\s*\{[^}]*Hono[^}]*\}\s*from\s*["']hono["'])"#).unwrap();
    if hono_re.is_match(source) {
        return DetectedFramework::Hono;
    }

    DetectedFramework::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_express_import() {
        assert_eq!(
            detect_framework(r#"import express from "express";"#),
            DetectedFramework::Express,
        );
    }

    #[test]
    fn test_detect_express_require() {
        assert_eq!(
            detect_framework(r#"const express = require("express");"#),
            DetectedFramework::Express,
        );
    }

    #[test]
    fn test_detect_fastify_import() {
        assert_eq!(
            detect_framework(r#"import Fastify from "fastify";"#),
            DetectedFramework::Fastify,
        );
    }

    #[test]
    fn test_detect_hono_import() {
        assert_eq!(
            detect_framework(r#"import { Hono } from "hono";"#),
            DetectedFramework::Hono,
        );
    }

    #[test]
    fn test_detect_unknown() {
        assert_eq!(
            detect_framework(r#"console.log("hello");"#),
            DetectedFramework::Unknown,
        );
    }

    #[test]
    fn test_detect_express_single_quotes() {
        assert_eq!(
            detect_framework(r#"const app = require('express');"#),
            DetectedFramework::Express,
        );
    }
}
