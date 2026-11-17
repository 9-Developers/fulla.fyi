package main

import (
	"testing"

	railway "github.com/railwayapp/railway-go-sdk"
)

func TestEnvironmentIsolation(t *testing.T) {
	for _, environment := range []string{"staging", "production"} {
		t.Run(environment, func(t *testing.T) {
			project := Railway(railway.Context{Environment: environment})
			var app railway.Service
			volumes := 0
			for _, resource := range project.Resources {
				switch value := resource.(type) {
				case railway.Service:
					app = value
				case railway.Resource:
					if value.Graph()["type"] == "volume" {
						volumes++
					}
				}
			}
			variables := app.Graph()["variables"].(map[string]any)
			databaseURL := variables["DATABASE_URL"].(map[string]any)
			if databaseURL["type"] != "reference" || databaseURL["resource"] != "database.Postgres" {
				t.Fatalf("database URL must reference this environment's database: %v", databaseURL)
			}
			if environment == "production" && volumes != 0 {
				t.Fatal("production must not import staging's volume")
			}
			if environment == "staging" && volumes != 1 {
				t.Fatal("staging must preserve its imported volume")
			}
		})
	}
}

func TestUnknownEnvironmentRejected(t *testing.T) {
	defer func() {
		if recover() == nil {
			t.Fatal("expected an unknown environment to be rejected")
		}
	}()
	Railway(railway.Context{Environment: "preview"})
}
