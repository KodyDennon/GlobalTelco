package core

import (
	"testing"
)

func TestParseSemVer(t *testing.T) {
	tests := []struct {
		input string
		want  SemVer
	}{
		{"0.1.0", SemVer{0, 1, 0, ""}},
		{"1.2.3", SemVer{1, 2, 3, ""}},
		{"v1.2.3", SemVer{1, 2, 3, ""}},
		{"0.5.1", SemVer{0, 5, 1, ""}},
		{"1.0.0-alpha.1", SemVer{1, 0, 0, "alpha.1"}},
		{"2.0.0-beta.3", SemVer{2, 0, 0, "beta.3"}},
	}

	for _, tt := range tests {
		t.Run(tt.input, func(t *testing.T) {
			got, err := ParseSemVer(tt.input)
			if err != nil {
				t.Fatalf("ParseSemVer(%q): %v", tt.input, err)
			}
			if got != tt.want {
				t.Errorf("ParseSemVer(%q) = %v, want %v", tt.input, got, tt.want)
			}
		})
	}
}

func TestSemVerString(t *testing.T) {
	tests := []struct {
		input SemVer
		want  string
	}{
		{SemVer{0, 1, 0, ""}, "0.1.0"},
		{SemVer{1, 2, 3, ""}, "1.2.3"},
		{SemVer{1, 0, 0, "alpha.1"}, "1.0.0-alpha.1"},
	}

	for _, tt := range tests {
		if got := tt.input.String(); got != tt.want {
			t.Errorf("SemVer(%v).String() = %q, want %q", tt.input, got, tt.want)
		}
	}
}

func TestSemVerBump(t *testing.T) {
	tests := []struct {
		input    SemVer
		bumpType BumpType
		want     string
	}{
		{SemVer{0, 1, 0, ""}, BumpPatch, "0.1.1"},
		{SemVer{0, 1, 0, ""}, BumpMinor, "0.2.0"},
		{SemVer{0, 1, 0, ""}, BumpMajor, "1.0.0"},
		{SemVer{1, 2, 3, ""}, BumpPatch, "1.2.4"},
		{SemVer{1, 2, 3, ""}, BumpMinor, "1.3.0"},
		{SemVer{1, 2, 3, ""}, BumpMajor, "2.0.0"},
	}

	for _, tt := range tests {
		got := tt.input.Bump(tt.bumpType)
		if got.String() != tt.want {
			t.Errorf("SemVer(%v).Bump(%v) = %q, want %q", tt.input, tt.bumpType, got.String(), tt.want)
		}
	}
}

func TestCategorizeCommit(t *testing.T) {
	tests := []struct {
		subject  string
		wantCat  CommitCategory
		wantSubj string
		wantBreaking bool
	}{
		{"feat: add new feature", CatFeature, "add new feature", false},
		{"fix: resolve crash", CatFix, "resolve crash", false},
		{"perf: optimize query", CatPerf, "optimize query", false},
		{"refactor: clean up code", CatRefactor, "clean up code", false},
		{"docs: update readme", CatDocs, "update readme", false},
		{"chore: bump deps", CatChore, "bump deps", false},
		{"feat!: breaking change", CatBreaking, "breaking change", true},
		{"feat(engine): add system", CatFeature, "add system", false},
		{"some random commit", CatOther, "some random commit", false},
	}

	for _, tt := range tests {
		t.Run(tt.subject, func(t *testing.T) {
			cc := CategorizeCommit("abc123", tt.subject)
			if cc.Category != tt.wantCat {
				t.Errorf("category: got %v, want %v", cc.Category, tt.wantCat)
			}
			if cc.Subject != tt.wantSubj {
				t.Errorf("subject: got %q, want %q", cc.Subject, tt.wantSubj)
			}
			if cc.Breaking != tt.wantBreaking {
				t.Errorf("breaking: got %v, want %v", cc.Breaking, tt.wantBreaking)
			}
		})
	}
}

func TestSuggestBump(t *testing.T) {
	tests := []struct {
		name    string
		commits []CategorizedCommit
		want    BumpType
	}{
		{
			"breaking change = major",
			[]CategorizedCommit{{Category: CatBreaking, Breaking: true}},
			BumpMajor,
		},
		{
			"feature = minor",
			[]CategorizedCommit{{Category: CatFeature}, {Category: CatFix}},
			BumpMinor,
		},
		{
			"only fixes = patch",
			[]CategorizedCommit{{Category: CatFix}, {Category: CatChore}},
			BumpPatch,
		},
		{
			"empty = patch",
			[]CategorizedCommit{},
			BumpPatch,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := SuggestBump(tt.commits)
			if got != tt.want {
				t.Errorf("SuggestBump = %v, want %v", got, tt.want)
			}
		})
	}
}

func TestParseBumpType(t *testing.T) {
	tests := []struct {
		input string
		want  BumpType
		err   bool
	}{
		{"patch", BumpPatch, false},
		{"minor", BumpMinor, false},
		{"major", BumpMajor, false},
		{"PATCH", BumpPatch, false},
		{"unknown", BumpPatch, true},
	}

	for _, tt := range tests {
		got, err := ParseBumpType(tt.input)
		if tt.err && err == nil {
			t.Errorf("ParseBumpType(%q): expected error", tt.input)
		}
		if !tt.err && err != nil {
			t.Errorf("ParseBumpType(%q): unexpected error: %v", tt.input, err)
		}
		if !tt.err && got != tt.want {
			t.Errorf("ParseBumpType(%q) = %v, want %v", tt.input, got, tt.want)
		}
	}
}
