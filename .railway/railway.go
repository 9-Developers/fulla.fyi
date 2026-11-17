package main

import railway "github.com/railwayapp/railway-go-sdk"

// Own only Fulla's resources, even when the environment contains other partials.
const Partial = "fulla.fyi"

func Railway(ctx railway.Context) railway.Project {
	ctx = railway.NewContext(ctx)
	switch ctx.Environment {
	case "staging", "production":
	default:
		panic("Fulla infrastructure supports only staging and production")
	}
	postgres := railway.Postgres("Postgres", map[string]any{
		"region": "us-east4-eqdc4a",
	})
	// Keep the existing persistent volume under the same partial as its database.
	volume := railway.Volume("postgres-volume", map[string]any{
		"region":            "us-east4-eqdc4a",
		"sizeMB":            5000,
		"allowOnlineResize": true,
		"alerts": map[string]any{
			"usage": map[string]any{
				"80": map[string]any{}, "95": map[string]any{}, "100": map[string]any{},
			},
		},
	})
	app := railway.ServiceNamed("fulla.fyi", railway.ServiceConfig{
		"build": map[string]any{
			"builder":      "RAILPACK",
			"buildCommand": "cargo build --release --locked && cp target/release/fulla_fyi fulla_fyi",
			"watchPatterns": []string{
				"src/**", "templates/**", "migrations/**", "Cargo.toml",
				"Cargo.lock", "build.rs", "railpack.json", "rust-toolchain.toml",
				".railway/**",
			},
		},
		"start":              "./fulla_fyi",
		"healthcheck":        "/health",
		"healthcheckTimeout": 300,
		"deploy": map[string]any{
			"restartPolicyType":       "ON_FAILURE",
			"restartPolicyMaxRetries": 10,
		},
		"env": map[string]any{
			"PORT":         "3000",
			"RUST_LOG":     "info",
			"DATABASE_URL": postgres.Env("DATABASE_URL"),
		},
	})
	resources := []any{postgres, app}
	// Production's database helper provisions its own volume. Only staging has
	// an imported volume whose existing settings need to be preserved.
	if ctx.IsEnvironment("staging") {
		resources = append(resources, volume)
	}
	return railway.ProjectNamed("fulla.fyi", resources)
}
