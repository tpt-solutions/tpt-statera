// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Proof-certificate assembly for `tpt-statera` (todo 1F).
//!
//! Work in progress: the certificate schema (1F.01) and the first-draft certificate
//! (1F.02–1F.04) are not yet implemented. This crate currently exports only the
//! scope caveat that every certificate must carry (spec §7, todo 1F.03).

/// Caveat text that must accompany any first-draft certificate, verbatim from spec §7 (todo 1F.03).
pub const ILLUSTRATIVE_CAVEAT: &str = "Phase 1's \"first-draft Proof Certificate\" is an illustrative artifact demonstrating the certificate format and workflow. It is not, by itself, a compliance claim, does not cover concurrent-access or multi-party netting scenarios (Section 8), and should not be represented to customers or auditors as a completed regulatory mapping until Phase 2 is delivered and Phase 3's independent audit has occurred.";
