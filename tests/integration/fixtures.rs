/// Test Fixtures and Data for Integration Tests
/// 
/// This module provides sample data, configurations, and utilities for testing.

use serde_json::json;
use std::collections::HashMap;

/// Sample code files for testing different languages
pub mod code_samples {
    /// Rust code samples
    pub const HELLO_WORLD_RUST: &str = r#"
fn main() {
    println!("Hello, world!");
}
"#;

    pub const SIMPLE_RUST_LIB: &str = r#"
pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
"#;

    pub const RUST_WEB_SERVER: &str = r#"
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        handle_connection(stream);
    }
}

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).unwrap();

    let response = "HTTP/1.1 200 OK\r\n\r\nHello, World!";
    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}
"#;

    /// Python code samples
    pub const HELLO_WORLD_PYTHON: &str = r#"
def main():
    print("Hello, world!")

if __name__ == "__main__":
    main()
"#;

    pub const PYTHON_WEB_SERVER: &str = r#"
from http.server import HTTPServer, BaseHTTPRequestHandler
import json

class SimpleHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header('Content-type', 'application/json')
        self.end_headers()
        
        response = {"message": "Hello, World!", "status": "success"}
        self.wfile.write(json.dumps(response).encode())

if __name__ == "__main__":
    server = HTTPServer(("localhost", 8080), SimpleHandler)
    print("Server running on http://localhost:8080")
    server.serve_forever()
"#;

    pub const PYTHON_DATA_ANALYSIS: &str = r#"
import pandas as pd
import numpy as np
import matplotlib.pyplot as plt

# Load and analyze data
def analyze_data(filename):
    df = pd.read_csv(filename)
    
    # Basic statistics
    print("Data shape:", df.shape)
    print("Summary statistics:")
    print(df.describe())
    
    # Create visualization
    plt.figure(figsize=(10, 6))
    df.hist(bins=20)
    plt.title("Data Distribution")
    plt.savefig("data_analysis.png")
    
    return df

if __name__ == "__main__":
    data = analyze_data("sample_data.csv")
"#;

    /// JavaScript/Node.js code samples
    pub const HELLO_WORLD_JS: &str = r#"
function main() {
    console.log("Hello, world!");
}

main();
"#;

    pub const NODE_WEB_SERVER: &str = r#"
const express = require('express');
const app = express();
const port = 3000;

app.use(express.json());

app.get('/', (req, res) => {
    res.json({ message: 'Hello, World!', timestamp: new Date().toISOString() });
});

app.get('/api/status', (req, res) => {
    res.json({ status: 'healthy', uptime: process.uptime() });
});

app.post('/api/data', (req, res) => {
    const { data } = req.body;
    res.json({ received: data, processed: true });
});

app.listen(port, () => {
    console.log(`Server running at http://localhost:${port}`);
});
"#;

    /// Go code samples
    pub const HELLO_WORLD_GO: &str = r#"
package main

import "fmt"

func main() {
    fmt.Println("Hello, world!")
}
"#;

    pub const GO_WEB_SERVER: &str = r#"
package main

import (
    "encoding/json"
    "fmt"
    "log"
    "net/http"
    "time"
)

type Response struct {
    Message   string    `json:"message"`
    Timestamp time.Time `json:"timestamp"`
}

func helloHandler(w http.ResponseWriter, r *http.Request) {
    response := Response{
        Message:   "Hello, World!",
        Timestamp: time.Now(),
    }
    
    w.Header().Set("Content-Type", "application/json")
    json.NewEncoder(w).Encode(response)
}

func main() {
    http.HandleFunc("/", helloHandler)
    
    fmt.Println("Server starting on :8080")
    log.Fatal(http.ListenAndServe(":8080", nil))
}
"#;
}

/// Configuration file samples
pub mod config_samples {
    pub const TOML_CONFIG: &str = r#"
[server]
host = "localhost"
port = 8080
max_connections = 100

[database]
url = "postgresql://user:pass@localhost/db"
max_pool_size = 20
timeout_seconds = 30

[logging]
level = "info"
file = "app.log"
max_size_mb = 100

[security]
jwt_secret = "your-secret-key"
session_timeout = 3600
"#;

    pub const JSON_CONFIG: &str = r#"
{
    "name": "sample-project",
    "version": "1.0.0",
    "description": "A sample project for testing",
    "main": "index.js",
    "scripts": {
        "start": "node index.js",
        "test": "jest",
        "build": "webpack --mode production",
        "dev": "nodemon index.js"
    },
    "dependencies": {
        "express": "^4.18.0",
        "dotenv": "^16.0.0",
        "cors": "^2.8.5"
    },
    "devDependencies": {
        "jest": "^28.0.0",
        "nodemon": "^2.0.0",
        "webpack": "^5.70.0"
    }
}
"#;

    pub const YAML_CONFIG: &str = r#"
apiVersion: v1
kind: ConfigMap
metadata:
  name: app-config
  namespace: default
data:
  database.url: "postgresql://localhost:5432/app"
  redis.url: "redis://localhost:6379"
  app.debug: "false"
  app.port: "8080"
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: sample-app
spec:
  replicas: 3
  selector:
    matchLabels:
      app: sample-app
  template:
    metadata:
      labels:
        app: sample-app
    spec:
      containers:
      - name: app
        image: sample-app:latest
        ports:
        - containerPort: 8080
"#;

    pub const ENV_CONFIG: &str = r#"
# Database configuration
DATABASE_URL=postgresql://user:password@localhost:5432/myapp
DATABASE_MAX_CONNECTIONS=20

# Redis configuration
REDIS_URL=redis://localhost:6379
REDIS_TTL=3600

# Application settings
APP_PORT=8080
APP_DEBUG=false
APP_SECRET_KEY=your-secret-key-here

# External APIs
OPENAI_API_KEY=sk-your-openai-key
GITHUB_TOKEN=ghp_your-github-token

# Logging
LOG_LEVEL=info
LOG_FILE=app.log
"#;
}

/// Test task definitions for planning tests
pub mod test_tasks {
    use serde_json::Value;

    pub const SIMPLE_TASK: &str = "Create a simple hello world program";
    
    pub const WEB_SERVER_TASK: &str = r#"
Create a basic web server with the following requirements:
1. Listen on port 8080
2. Serve a JSON response at the root endpoint
3. Include proper error handling
4. Add logging for requests
"#;

    pub const DATA_PROCESSING_TASK: &str = r#"
Build a data processing pipeline that:
1. Reads CSV files from an input directory
2. Performs data validation and cleaning
3. Applies statistical analysis
4. Generates summary reports
5. Saves results to an output directory
"#;

    pub const CRUD_API_TASK: &str = r#"
Implement a RESTful CRUD API for user management:
1. User model with name, email, and timestamps
2. GET /users - list all users
3. GET /users/:id - get specific user
4. POST /users - create new user
5. PUT /users/:id - update user
6. DELETE /users/:id - delete user
7. Input validation and error handling
8. Database integration with migrations
"#;

    pub const REFACTORING_TASK: &str = r#"
Refactor the existing codebase to improve maintainability:
1. Extract common functionality into utility modules
2. Implement proper error handling throughout
3. Add comprehensive unit tests
4. Improve code documentation
5. Optimize performance bottlenecks
6. Follow language-specific best practices
"#;

    pub const MICROSERVICE_TASK: &str = r#"
Design and implement a microservice architecture:
1. User authentication service
2. Product catalog service  
3. Order processing service
4. API gateway for routing
5. Service discovery mechanism
6. Health check endpoints
7. Logging and monitoring
8. Docker containerization
9. Kubernetes deployment configs
"#;

    /// Get task arguments for different complexity levels
    pub fn get_task_args(task: &str, complexity: &str) -> Value {
        serde_json::json!({
            "task": task,
            "complexity": complexity,
            "requirements": get_requirements_for_complexity(complexity),
            "estimated_time": get_estimated_time(complexity),
            "technologies": get_suggested_technologies(task)
        })
    }

    fn get_requirements_for_complexity(complexity: &str) -> Vec<&str> {
        match complexity {
            "simple" => vec!["Basic functionality", "Minimal error handling"],
            "moderate" => vec![
                "Core functionality", 
                "Error handling", 
                "Basic tests", 
                "Documentation"
            ],
            "complex" => vec![
                "Full feature set",
                "Comprehensive error handling",
                "Unit and integration tests",
                "Performance optimization",
                "Security considerations",
                "Detailed documentation"
            ],
            _ => vec!["Standard requirements"]
        }
    }

    fn get_estimated_time(complexity: &str) -> &str {
        match complexity {
            "simple" => "1-2 hours",
            "moderate" => "4-8 hours", 
            "complex" => "1-3 days",
            _ => "Variable"
        }
    }

    fn get_suggested_technologies(task: &str) -> Vec<&str> {
        if task.contains("web server") || task.contains("API") {
            vec!["HTTP server", "JSON handling", "Routing", "Middleware"]
        } else if task.contains("data") {
            vec!["CSV parsing", "Data validation", "Statistics", "File I/O"]
        } else if task.contains("microservice") {
            vec!["Docker", "REST API", "Database", "Service mesh"]
        } else {
            vec!["General programming"]
        }
    }
}

/// File structure templates for different project types
pub mod project_templates {
    use std::collections::HashMap;

    pub fn get_rust_project_structure() -> HashMap<String, String> {
        let mut files = HashMap::new();
        
        files.insert("Cargo.toml".to_string(), r#"
[package]
name = "sample-project"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
"#.to_string());

        files.insert("src/main.rs".to_string(), super::code_samples::HELLO_WORLD_RUST.to_string());
        files.insert("src/lib.rs".to_string(), super::code_samples::SIMPLE_RUST_LIB.to_string());
        files.insert("README.md".to_string(), "# Sample Rust Project\n\nA sample project for testing.".to_string());
        
        files
    }

    pub fn get_node_project_structure() -> HashMap<String, String> {
        let mut files = HashMap::new();
        
        files.insert("package.json".to_string(), super::config_samples::JSON_CONFIG.to_string());
        files.insert("index.js".to_string(), super::code_samples::NODE_WEB_SERVER.to_string());
        files.insert(".env".to_string(), super::config_samples::ENV_CONFIG.to_string());
        files.insert("README.md".to_string(), "# Sample Node.js Project\n\nA sample project for testing.".to_string());
        
        files
    }

    pub fn get_python_project_structure() -> HashMap<String, String> {
        let mut files = HashMap::new();
        
        files.insert("main.py".to_string(), super::code_samples::HELLO_WORLD_PYTHON.to_string());
        files.insert("server.py".to_string(), super::code_samples::PYTHON_WEB_SERVER.to_string());
        files.insert("requirements.txt".to_string(), r#"
fastapi==0.68.0
uvicorn==0.15.0
pandas==1.3.0
numpy==1.21.0
requests==2.26.0
"#.to_string());
        files.insert("README.md".to_string(), "# Sample Python Project\n\nA sample project for testing.".to_string());
        
        files
    }
}

/// Mock responses for different provider scenarios
pub mod mock_responses {
    pub fn get_planning_responses() -> Vec<String> {
        vec![
            r#"## Implementation Plan

### Step 1: Project Setup
- Initialize new project with appropriate build tool
- Set up directory structure
- Configure dependencies

### Step 2: Core Implementation  
- Implement main functionality
- Add error handling
- Create utility functions

### Step 3: Testing & Documentation
- Write unit tests
- Add integration tests
- Create documentation

### Step 4: Deployment
- Build production version
- Configure deployment environment
- Deploy application"#.to_string(),

            r#"## Development Approach

1. **Analysis Phase**
   - Understand requirements
   - Identify key components
   - Plan architecture

2. **Implementation Phase**
   - Start with basic structure
   - Implement core features
   - Add advanced functionality

3. **Testing Phase**
   - Unit testing
   - Integration testing
   - End-to-end testing

4. **Optimization Phase**
   - Performance tuning
   - Security hardening
   - Documentation"#.to_string(),

            r#"## Quick Implementation Guide

**Immediate Steps:**
1. Create project skeleton
2. Implement basic functionality
3. Add error handling
4. Write basic tests

**Next Steps:**
1. Enhance features
2. Improve performance
3. Add comprehensive tests
4. Create documentation

**Final Steps:**
1. Code review
2. Security audit
3. Performance testing
4. Deployment preparation"#.to_string(),
        ]
    }

    pub fn get_editing_responses() -> Vec<String> {
        vec![
            "I'll help you edit the file. Here are the changes I'm making...".to_string(),
            "Editing the file with the requested changes. The modifications include...".to_string(),
            "Applying edits to the file. The updated content will...".to_string(),
        ]
    }

    pub fn get_error_responses() -> Vec<String> {
        vec![
            "I encountered an error while processing your request. Please try again.".to_string(),
            "There was an issue with the operation. Error details: ...".to_string(),
            "The request could not be completed due to an error.".to_string(),
        ]
    }
}

/// Predefined test scenarios for comprehensive testing
pub mod test_scenarios {
    use serde_json::Value;

    #[derive(Debug, Clone)]
    pub struct TestScenario {
        pub name: String,
        pub description: String,
        pub steps: Vec<ScenarioStep>,
        pub expected_outcomes: Vec<String>,
    }

    #[derive(Debug, Clone)]
    pub struct ScenarioStep {
        pub command: String,
        pub args: Value,
        pub expected_success: bool,
        pub delay_ms: u64,
    }

    pub fn get_hello_world_scenario() -> TestScenario {
        TestScenario {
            name: "Hello World Development".to_string(),
            description: "Complete workflow for creating a hello world program".to_string(),
            steps: vec![
                ScenarioStep {
                    command: "plan".to_string(),
                    args: serde_json::json!({"task": "Create a hello world program in Rust"}),
                    expected_success: true,
                    delay_ms: 100,
                },
                ScenarioStep {
                    command: "edit".to_string(),
                    args: serde_json::json!({
                        "path": "src/main.rs",
                        "content": super::code_samples::HELLO_WORLD_RUST
                    }),
                    expected_success: true,
                    delay_ms: 50,
                },
                ScenarioStep {
                    command: "diff".to_string(),
                    args: serde_json::json!({"path": "src/main.rs"}),
                    expected_success: true,
                    delay_ms: 50,
                },
            ],
            expected_outcomes: vec![
                "Plan should be generated".to_string(),
                "File should be created".to_string(),
                "Diff should show changes".to_string(),
            ],
        }
    }

    pub fn get_web_server_scenario() -> TestScenario {
        TestScenario {
            name: "Web Server Development".to_string(),
            description: "Complete workflow for creating a web server".to_string(),
            steps: vec![
                ScenarioStep {
                    command: "plan".to_string(),
                    args: serde_json::json!({"task": super::test_tasks::WEB_SERVER_TASK}),
                    expected_success: true,
                    delay_ms: 200,
                },
                ScenarioStep {
                    command: "edit".to_string(),
                    args: serde_json::json!({
                        "path": "src/server.rs",
                        "content": super::code_samples::RUST_WEB_SERVER
                    }),
                    expected_success: true,
                    delay_ms: 100,
                },
                ScenarioStep {
                    command: "edit".to_string(),
                    args: serde_json::json!({
                        "path": "Cargo.toml",
                        "content": r#"
[package]
name = "web-server"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1.0", features = ["full"] }
"#
                    }),
                    expected_success: true,
                    delay_ms: 50,
                },
            ],
            expected_outcomes: vec![
                "Detailed plan should be generated".to_string(),
                "Server file should be created".to_string(),
                "Cargo.toml should be configured".to_string(),
            ],
        }
    }

    pub fn get_error_handling_scenario() -> TestScenario {
        TestScenario {
            name: "Error Handling Test".to_string(),
            description: "Test error handling and recovery".to_string(),
            steps: vec![
                ScenarioStep {
                    command: "edit".to_string(),
                    args: serde_json::json!({
                        "path": "/invalid/path/file.txt",
                        "content": "This should fail"
                    }),
                    expected_success: false,
                    delay_ms: 50,
                },
                ScenarioStep {
                    command: "nonexistent_command".to_string(),
                    args: serde_json::json!({}),
                    expected_success: false,
                    delay_ms: 50,
                },
                ScenarioStep {
                    command: "plan".to_string(),
                    args: serde_json::json!({"task": "Recovery test"}),
                    expected_success: true,
                    delay_ms: 100,
                },
            ],
            expected_outcomes: vec![
                "First edit should fail due to invalid path".to_string(),
                "Invalid command should fail".to_string(),
                "Recovery with valid command should succeed".to_string(),
            ],
        }
    }

    pub fn get_all_scenarios() -> Vec<TestScenario> {
        vec![
            get_hello_world_scenario(),
            get_web_server_scenario(),
            get_error_handling_scenario(),
        ]
    }
}

/// Utility functions for test data generation
pub mod test_data_gen {
    use rand::Rng;

    pub fn generate_random_string(length: usize) -> String {
        const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                                abcdefghijklmnopqrstuvwxyz\
                                0123456789";
        let mut rng = rand::thread_rng();
        (0..length)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }

    pub fn generate_file_content(size_kb: usize) -> String {
        let content_per_line = "This is a test line with some content to reach the desired size.\n";
        let lines_needed = (size_kb * 1024) / content_per_line.len();
        content_per_line.repeat(lines_needed)
    }

    pub fn generate_test_tasks(count: usize) -> Vec<String> {
        let task_templates = vec![
            "Create a {} application",
            "Implement {} functionality",
            "Build a {} service",
            "Design a {} system",
            "Develop a {} tool",
        ];

        let technologies = vec![
            "web", "mobile", "desktop", "CLI", "API", "database", 
            "machine learning", "data processing", "monitoring", "testing"
        ];

        let mut tasks = Vec::new();
        let mut rng = rand::thread_rng();

        for _ in 0..count {
            let template = task_templates[rng.gen_range(0..task_templates.len())];
            let tech = technologies[rng.gen_range(0..technologies.len())];
            tasks.push(template.replace("{}", tech));
        }

        tasks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_samples_are_valid() {
        // Test that code samples are not empty
        assert!(!code_samples::HELLO_WORLD_RUST.trim().is_empty());
        assert!(!code_samples::HELLO_WORLD_PYTHON.trim().is_empty());
        assert!(!code_samples::NODE_WEB_SERVER.trim().is_empty());
    }

    #[test]
    fn test_config_samples_are_valid() {
        // Test that config samples are not empty and properly formatted
        assert!(!config_samples::TOML_CONFIG.trim().is_empty());
        assert!(!config_samples::JSON_CONFIG.trim().is_empty());
        
        // Test JSON is valid
        let _: serde_json::Value = serde_json::from_str(config_samples::JSON_CONFIG).unwrap();
    }

    #[test]
    fn test_project_templates() {
        let rust_project = project_templates::get_rust_project_structure();
        assert!(rust_project.contains_key("Cargo.toml"));
        assert!(rust_project.contains_key("src/main.rs"));
        
        let node_project = project_templates::get_node_project_structure();
        assert!(node_project.contains_key("package.json"));
        assert!(node_project.contains_key("index.js"));
    }

    #[test]
    fn test_scenario_generation() {
        let scenarios = test_scenarios::get_all_scenarios();
        assert!(!scenarios.is_empty());
        
        for scenario in scenarios {
            assert!(!scenario.name.is_empty());
            assert!(!scenario.steps.is_empty());
        }
    }

    #[test]
    fn test_data_generation() {
        let random_string = test_data_gen::generate_random_string(10);
        assert_eq!(random_string.len(), 10);
        
        let file_content = test_data_gen::generate_file_content(1); // 1KB
        assert!(file_content.len() >= 1000); // Approximately 1KB
        
        let tasks = test_data_gen::generate_test_tasks(5);
        assert_eq!(tasks.len(), 5);
        for task in tasks {
            assert!(!task.is_empty());
        }
    }
}