package main

import (
	"fmt"
	"log"
	"os"

	"github.com/linuxiano85/NovaPcSuite/internal/backup"
)

func main() {
	if len(os.Args) < 2 {
		fmt.Println("NovaPcSuite Backup Engine")
		fmt.Println("Usage: novapc <command> [options]")
		fmt.Println("Commands:")
		fmt.Println("  scan <path>     - Scan directory for backup")
		fmt.Println("  plan <path>     - Create backup plan")  
		fmt.Println("  run <path>      - Execute backup")
		os.Exit(1)
	}

	command := os.Args[1]
	
	switch command {
	case "scan":
		if len(os.Args) < 3 {
			log.Fatal("scan command requires a path")
		}
		path := os.Args[2]
		engine := backup.NewEngine("./backups")
		if err := engine.Scan(path); err != nil {
			log.Fatal("Scan failed:", err)
		}
	case "plan":
		if len(os.Args) < 3 {
			log.Fatal("plan command requires a path")
		}
		path := os.Args[2]
		engine := backup.NewEngine("./backups")
		if err := engine.Plan(path); err != nil {
			log.Fatal("Plan failed:", err)
		}
	case "run":
		if len(os.Args) < 3 {
			log.Fatal("run command requires a path")
		}
		path := os.Args[2]
		engine := backup.NewEngine("./backups")
		if err := engine.Run(path); err != nil {
			log.Fatal("Backup failed:", err)
		}
	default:
		log.Fatal("Unknown command:", command)
	}
}