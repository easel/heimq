package main

import (
	"os"
	"path/filepath"
	"testing"
)

// @covers US-005-AC1
// @covers US-005-AC3
func TestDiffReportsStructuredMismatchAndZeroDiffSuccess(t *testing.T) {
	ex := &Exemptions{}
	heimq := []Observation{{
		Workload: "produce_fetch_roundtrip",
		Step:     0,
		Event:    recordConsumed([]byte("k"), []byte("heimq"), 0, 0),
	}}
	oracle := []Observation{{
		Workload: "produce_fetch_roundtrip",
		Step:     0,
		Event:    recordConsumed([]byte("k"), []byte("oracle"), 0, 0),
	}}

	diffs := diff("produce_fetch_roundtrip", "redpanda", heimq, oracle, ex)
	if len(diffs) != 1 {
		t.Fatalf("expected one structured diff, got %d", len(diffs))
	}
	if diffs[0].Workload != "produce_fetch_roundtrip" ||
		diffs[0].Oracle != "redpanda" ||
		diffs[0].Step != 0 ||
		diffs[0].Field != "record.value" ||
		diffs[0].HeimqValue != "heimq" ||
		diffs[0].OracleValue != "oracle" ||
		diffs[0].Divergence != "value_mismatch" {
		t.Fatalf("unexpected diff record: %#v", diffs[0])
	}

	diffs = diff("produce_fetch_roundtrip", "redpanda", heimq, heimq, ex)
	if len(diffs) != 0 {
		t.Fatalf("expected zero diffs for matching observations, got %#v", diffs)
	}
}

// @covers US-005-AC6
func TestLoadExemptionsRejectsDuplicateIDs(t *testing.T) {
	path := writeTOML(t, `
[[exemption]]
id = "dup"
field = "record.offset"
scope = "all"
oracle = "all"
reason = "first"
prd_ref = "PRD non-goal #1"
status = "active"

[[exemption]]
id = "dup"
field = "record.value"
scope = "all"
oracle = "all"
reason = "second"
prd_ref = "PRD non-goal #1"
status = "active"
`)

	if _, err := loadExemptions(path); err == nil {
		t.Fatal("expected duplicate exemption ID to be rejected")
	}
}

// @covers US-005-AC6
func TestLoadExemptionsRejectsEmptyPRDRef(t *testing.T) {
	path := writeTOML(t, `
[[exemption]]
id = "missing-prd"
field = "record.offset"
scope = "all"
oracle = "all"
reason = "must cite the governing PRD non-goal"
prd_ref = ""
status = "active"
`)

	if _, err := loadExemptions(path); err == nil {
		t.Fatal("expected empty prd_ref to be rejected")
	}
}

func writeTOML(t *testing.T, contents string) string {
	t.Helper()

	path := filepath.Join(t.TempDir(), "exemptions.toml")
	if err := os.WriteFile(path, []byte(contents), 0o644); err != nil {
		t.Fatalf("write exemptions.toml: %v", err)
	}
	return path
}
