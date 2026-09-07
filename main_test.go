package main

import (
	"encoding/json"
	"os/exec"
	"slices"
	"strings"
	"testing"
)

func TestDescribeValid(t *testing.T) {
	var descriptor map[string]any
	if err := json.Unmarshal([]byte(describe), &descriptor); err != nil {
		t.Fatalf("descriptor is not valid JSON: %v", err)
	}
	if descriptor["name"] != "bash" {
		t.Errorf("name = %v, want bash", descriptor["name"])
	}
	description, _ := descriptor["description"].(string)
	if strings.TrimSpace(description) == "" {
		t.Error("description is empty")
	}
	params, ok := descriptor["parameters"].(map[string]any)
	if !ok {
		t.Fatalf("parameters is not an object: %T", descriptor["parameters"])
	}
	props, ok := params["properties"].(map[string]any)
	if !ok {
		t.Fatalf("properties is not an object: %T", params["properties"])
	}
	if _, ok := props["command"]; !ok {
		t.Error("parameters.properties is missing command")
	}
	if _, ok := props["timeout"]; !ok {
		t.Error("parameters.properties is missing timeout")
	}
	if required, ok := params["required"].([]any); !ok || len(required) != 1 || required[0] != "command" {
		t.Errorf("required = %v, want [command]", params["required"])
	}
	if descriptor["snippet"] == "" {
		t.Error("snippet is empty")
	}
}

func TestTailBounded(t *testing.T) {
	if got := tail(strings.Repeat("x", maxOutput+100)); len(got) != maxOutput {
		t.Errorf("tail length = %d, want %d", len(got), maxOutput)
	}
	if got := tail(strings.Repeat("é", maxOutput/2)); len(got) != maxOutput {
		// Each é is 2 bytes; a rune boundary round-trips exactly.
		t.Errorf("tail of multibyte runes length = %d, want %d", len(got), maxOutput)
	}
	if got := tail("hello"); got != "hello" {
		t.Errorf("tail of short output = %q, want %q", got, "hello")
	}
	// A cut in the middle of a multibyte rune must not break it.
	full := strings.Repeat("a", maxOutput-1) + "é" + strings.Repeat("b", 20)
	got := tail(full)
	if len(got) > maxOutput {
		t.Errorf("tail exceeded maxOutput: %d", len(got))
	}
	if strings.Contains(got, "\uFFFD") {
		t.Errorf("tail broke a multibyte rune: %q", got)
	}
}

func TestCleanEnvMinimalAndSecretFree(t *testing.T) {
	t.Setenv("SLACK_BOT_TOKEN", "xoxb-secret")
	t.Setenv("DEEPSEEK_API_KEY", "sk-secret")
	t.Setenv("DATABASE_URL", "postgres://secret")
	t.Setenv("BOT_ROOT", "/bot")

	env := cleanEnv("/tmp/ws")
	keys := make([]string, 0, len(env))
	for _, entry := range env {
		key, _, _ := strings.Cut(entry, "=")
		keys = append(keys, key)
	}
	want := []string{"PATH", "HOME", "LANG", "TERM", "BOT_ROOT"}
	slices.Sort(keys)
	slices.Sort(want)
	if !slices.Equal(keys, want) {
		t.Errorf("cleanEnv keys = %v, want %v", keys, want)
	}
	for _, entry := range env {
		if strings.Contains(entry, "secret") {
			t.Errorf("cleanEnv leaked a secret: %q", entry)
		}
	}
}

func TestStatusString(t *testing.T) {
	if got := statusString(nil); got != "" {
		t.Errorf("statusString(nil) = %q, want empty", got)
	}
	// A real non-zero exit yields an ExitError; format must match the contract.
	err := exec.Command("/bin/sh", "-c", "exit 3").Run()
	if err == nil {
		t.Fatal("expected an exit error")
	}
	if got := statusString(err); got != "exit status: 3" {
		t.Errorf("statusString = %q, want \"exit status: 3\"", got)
	}
}

func TestCappedBuffer(t *testing.T) {
	var b cappedBuffer
	chunk := strings.Repeat("x", 1024)
	for i := 0; i < 64; i++ {
		if _, err := b.Write([]byte(chunk)); err != nil {
			t.Fatalf("Write: %v", err)
		}
	}
	if got := len(b.String()); got != maxOutput {
		t.Errorf("cappedBuffer length = %d, want %d", got, maxOutput)
	}
}
