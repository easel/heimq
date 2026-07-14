package main

import (
	"fmt"
	"os"
	"reflect"

	"github.com/BurntSushi/toml"
)

// Observation mirrors the schema the Rust harness emitted, so JSONL output is
// comparable across runners.
type Observation struct {
	Workload string
	Step     int
	Event    map[string]any
}

// @covers US-005-AC2
func recordConsumed(key, value []byte, partition int32, offset int64) map[string]any {
	return map[string]any{
		"type":      "RecordConsumed",
		"key":       key,
		"value":     value,
		"headers":   []any{},
		"partition": partition,
		"offset":    offset,
		"timestamp": int64(0), // normalized away
	}
}

func errorCode(api string, code int16) map[string]any {
	return map[string]any{"type": "ErrorCode", "api": api, "code": code}
}

func groupState(groupID, state string, memberCount int) map[string]any {
	return map[string]any{
		"type":         "GroupState",
		"group_id":     groupID,
		"state":        state,
		"member_count": memberCount,
	}
}

// DiffRecord is one diverging field. Field names match the Rust serde output.
type DiffRecord struct {
	Workload    string  `json:"workload"`
	Oracle      string  `json:"oracle"`
	Step        int     `json:"step"`
	Field       string  `json:"field"`
	HeimqValue  any     `json:"heimq_value"`
	OracleValue any     `json:"oracle_value"`
	Divergence  string  `json:"divergence"`
	Exemption   *string `json:"exemption"`
}

// comparedFields lists, per event type, the (event key, diff field name) pairs.
// Order matches the Python runner.
// @covers US-005-AC1
var comparedFields = map[string][][2]string{
	"RecordConsumed": {
		{"key", "record.key"},
		{"value", "record.value"},
		{"partition", "record.partition"},
		{"offset", "record.offset"},
	},
	"ErrorCode": {{"api", "error_code.api"}, {"code", "error_code.code"}},
	"GroupState": {
		{"state", "group_state.state"},
		{"member_count", "group_state.member_count"},
	},
	"TxnOutcome": {
		{"committed", "txn_outcome.committed"},
		{"records_visible", "txn_outcome.records_visible"},
	},
}

// jsonValue renders bytes as a string, matching the Rust from_utf8_lossy output.
func jsonValue(v any) any {
	if b, ok := v.([]byte); ok {
		if b == nil {
			return nil
		}
		return string(b)
	}
	return v
}

type Exemption struct {
	ID     string `toml:"id"`
	Field  string `toml:"field"`
	Scope  string `toml:"scope"`
	Oracle string `toml:"oracle"`
	Reason string `toml:"reason"`
	PRDRef string `toml:"prd_ref"`
	Status string `toml:"status"`
}

type Exemptions struct{ Entries []Exemption }

// find matches on field, active status, scope ("all" or the workload), and
// oracle ("all" or the reference broker).
// @covers US-005-AC6
func (e *Exemptions) find(field, workload, oracle string) *string {
	for i := range e.Entries {
		x := &e.Entries[i]
		if x.Field == field && x.Status == "active" &&
			(x.Scope == "all" || x.Scope == workload) &&
			(x.Oracle == "all" || x.Oracle == oracle) {
			return &x.ID
		}
	}
	return nil
}

// @covers US-005-AC6
func loadExemptions(path string) (*Exemptions, error) {
	var doc struct {
		Exemption []Exemption `toml:"exemption"`
	}
	if _, err := os.Stat(path); os.IsNotExist(err) {
		return &Exemptions{}, nil
	}
	if _, err := toml.DecodeFile(path, &doc); err != nil {
		return nil, fmt.Errorf("parse %s: %w", path, err)
	}
	seen := map[string]bool{}
	for _, x := range doc.Exemption {
		if x.Oracle == "" {
			x.Oracle = "all"
		}
		if seen[x.ID] {
			return nil, fmt.Errorf("duplicate exemption id: %s", x.ID)
		}
		seen[x.ID] = true
		if x.PRDRef == "" {
			return nil, fmt.Errorf("exemption %q has empty prd_ref", x.ID)
		}
	}
	return &Exemptions{Entries: doc.Exemption}, nil
}

// diff compares two normalized observation streams: heimq against one oracle.
// @covers US-005-AC1
// @covers US-005-AC3
// @covers US-005-AC6
func diff(workload, oracle string, heimq, oracleObs []Observation, ex *Exemptions) []DiffRecord {
	var out []DiffRecord
	shared := min(len(heimq), len(oracleObs))

	for _, o := range heimq[shared:] {
		out = append(out, DiffRecord{
			Workload: workload, Oracle: oracle, Step: o.Step, Field: "observation",
			HeimqValue: o.Event["type"], OracleValue: nil, Divergence: "extra_in_heimq",
		})
	}
	for _, o := range oracleObs[shared:] {
		out = append(out, DiffRecord{
			Workload: workload, Oracle: oracle, Step: o.Step, Field: "observation",
			HeimqValue: nil, OracleValue: o.Event["type"], Divergence: "missing_in_heimq",
		})
	}

	for i := 0; i < shared; i++ {
		h, r := heimq[i].Event, oracleObs[i].Event
		step := heimq[i].Step
		if h["type"] != r["type"] {
			out = append(out, DiffRecord{
				Workload: workload, Oracle: oracle, Step: step, Field: "event_type",
				HeimqValue: h["type"], OracleValue: r["type"], Divergence: "value_mismatch",
			})
			continue
		}
		for _, pair := range comparedFields[h["type"].(string)] {
			key, field := pair[0], pair[1]
			if reflect.DeepEqual(h[key], r[key]) {
				continue
			}
			out = append(out, DiffRecord{
				Workload: workload, Oracle: oracle, Step: step, Field: field,
				HeimqValue: jsonValue(h[key]), OracleValue: jsonValue(r[key]),
				Divergence: "value_mismatch", Exemption: ex.find(field, workload, oracle),
			})
		}
	}
	return out
}
