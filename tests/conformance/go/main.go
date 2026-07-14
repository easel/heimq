package main

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"time"

	"github.com/twmb/franz-go/pkg/kadm"
	"github.com/twmb/franz-go/pkg/kgo"
)

const exemptionsPath = "/conformance/exemptions.toml"

type Target struct {
	Name      string
	Bootstrap string
}

// waitReady polls metadata until the broker answers. KRaft startup lags the log
// line the old harness waited on, so retry rather than fail on the first probe.
func waitReady(ctx context.Context, t Target) error {
	var last error
	for i := 0; i < 60; i++ {
		cl, err := kgo.NewClient(kgo.SeedBrokers(t.Bootstrap))
		if err != nil {
			last = err
		} else {
			pctx, cancel := context.WithTimeout(ctx, 5*time.Second)
			_, err = kadm.NewClient(cl).ListTopics(pctx)
			cancel()
			cl.Close()
			if err == nil {
				return nil
			}
			last = err
		}
		time.Sleep(time.Second)
	}
	return fmt.Errorf("%s not ready at %s: %w", t.Name, t.Bootstrap, last)
}

func mustEnv(k string) string {
	v := os.Getenv(k)
	if v == "" {
		fmt.Fprintf(os.Stderr, "missing required env var %s\n", k)
		os.Exit(2)
	}
	return v
}

// @covers US-005-AC1
// @covers US-005-AC3
func run() error {
	ctx := context.Background()

	heimq := Target{"heimq", mustEnv("HEIMQ_BOOTSTRAP")}
	oracles := []Target{
		{"kafka", mustEnv("KAFKA_BOOTSTRAP")},
		{"redpanda", mustEnv("REDPANDA_BOOTSTRAP")},
	}

	ex, err := loadExemptions(exemptionsPath)
	if err != nil {
		return err
	}

	fmt.Println("waiting for brokers...")
	for _, t := range append([]Target{heimq}, oracles...) {
		if err := waitReady(ctx, t); err != nil {
			return err
		}
	}
	fmt.Println("brokers ready")

	outDir := os.Getenv("OUT_DIR")
	if outDir == "" {
		outDir = "/out"
	}
	if err := os.MkdirAll(outDir, 0o755); err != nil {
		return err
	}

	anyFail := false
	for _, w := range allWorkloads() {
		heimqObs, err := w.Run(ctx, heimq.Bootstrap)
		if err != nil {
			return fmt.Errorf("%s on heimq: %w", w.Name, err)
		}

		for _, oracle := range oracles {
			oracleObs, err := w.Run(ctx, oracle.Bootstrap)
			if err != nil {
				return fmt.Errorf("%s on %s: %w", w.Name, oracle.Name, err)
			}

			diffs := diff(w.Name, oracle.Name, heimqObs, oracleObs, ex)
			var unmatched []DiffRecord
			for _, d := range diffs {
				if d.Exemption == nil {
					unmatched = append(unmatched, d)
				}
			}

			ts := time.Now().UTC().Format("20060102T150405Z")
			path := filepath.Join(outDir, fmt.Sprintf("%s-%s-%s.jsonl", ts, oracle.Name, w.Name))
			f, err := os.Create(path)
			if err != nil {
				return err
			}
			enc := json.NewEncoder(f)
			for _, d := range diffs {
				if err := enc.Encode(d); err != nil {
					f.Close()
					return err
				}
			}
			f.Close()

			if len(unmatched) == 0 {
				fmt.Printf("[PASS] %s vs %s: 0 diffs\n", w.Name, oracle.Name)
			} else {
				fmt.Printf("[FAIL] %s vs %s: %d unmatched diffs\n",
					w.Name, oracle.Name, len(unmatched))
				for _, d := range unmatched {
					b, _ := json.Marshal(d)
					fmt.Printf("  %s\n", b)
				}
				anyFail = true
			}
		}
	}

	if anyFail {
		return fmt.Errorf("one or more parity workloads had unmatched diffs")
	}
	return nil
}

func main() {
	if err := run(); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
