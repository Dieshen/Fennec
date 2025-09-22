# Fennec Tutorials

Step-by-step tutorials for common development workflows with Fennec AI Assistant.

## Table of Contents

1. [Tutorial 1: Your First Fennec Session](#tutorial-1-your-first-fennec-session)
2. [Tutorial 2: Building a Web Application](#tutorial-2-building-a-web-application)
3. [Tutorial 3: Bug Hunting and Fixing](#tutorial-3-bug-hunting-and-fixing)
4. [Tutorial 4: Code Refactoring](#tutorial-4-code-refactoring)
5. [Tutorial 5: Security Review and Hardening](#tutorial-5-security-review-and-hardening)
6. [Tutorial 6: Advanced Memory Management](#tutorial-6-advanced-memory-management)

## Tutorial 1: Your First Fennec Session

**Goal**: Get familiar with Fennec's interface and basic commands.

**Time**: 15 minutes

### Prerequisites
- Fennec installed and configured
- OpenAI API key set up
- A simple project directory

### Step 1: Start Fennec

```bash
# Create a sample project
mkdir ~/tutorial-project
cd ~/tutorial-project
echo "console.log('Hello, World!');" > hello.js

# Start Fennec
fennec
```

You'll see Fennec's TUI with three panels:
- Left: Chat interface
- Right: Preview/results panel
- Bottom: Status bar showing sandbox level

### Step 2: Your First Plan

In the chat panel, type:
```
plan "Add error handling to the hello.js file"
```

**What happens:**
- Fennec analyzes your current file
- Creates a structured implementation plan
- Shows the plan in the preview panel

**Expected output:**
```
Implementation Plan: Add Error Handling to hello.js

1. Analysis of Current Code
   - Simple console.log statement
   - No error handling present
   - Node.js environment assumed

2. Proposed Enhancements
   - Add try-catch block around main logic
   - Handle potential console access errors
   - Add process error event handlers
   - Include graceful shutdown handling

3. Implementation Steps
   - Wrap existing code in try-catch
   - Add error logging functionality
   - Test error scenarios
```

### Step 3: Make Your First Edit

Type:
```
edit hello.js "Add try-catch error handling around the console.log"
```

**What happens:**
- Fennec generates a diff preview
- Shows you exactly what will change
- Waits for your confirmation

**Expected preview:**
```diff
- console.log('Hello, World!');
+ try {
+     console.log('Hello, World!');
+ } catch (error) {
+     console.error('Error occurred:', error.message);
+     process.exit(1);
+ }
```

Press Enter to confirm the change.

### Step 4: View the Changes

Type:
```
diff hello.js
```

This shows you the complete diff of what changed in your file.

### Step 5: Test Your Changes

Type:
```
run "node hello.js"
```

**What happens:**
- Fennec executes the command safely
- Shows the output in the preview panel
- Logs the command execution in audit trail

### Step 6: Summarize Your Session

Type:
```
summarize
```

**What happens:**
- Fennec creates a summary of what you accomplished
- Offers to save it to memory files
- Updates the session transcript

**Congratulations!** You've completed your first Fennec session. You've learned to:
- ✅ Plan implementations
- ✅ Make file edits with previews
- ✅ Execute commands safely
- ✅ View diffs and changes
- ✅ Summarize sessions

## Tutorial 2: Building a Web Application

**Goal**: Build a simple web application from scratch using Fennec.

**Time**: 45 minutes

**What you'll build**: A task management API with Express.js

### Prerequisites
- Node.js installed
- Empty project directory

### Step 1: Project Setup

```bash
mkdir ~/fennec-todo-api
cd ~/fennec-todo-api
fennec
```

### Step 2: Initialize the Project

Type:
```
plan "Create a task management REST API with Express.js, including CRUD operations for tasks"
```

Fennec will create a comprehensive plan covering:
- Project initialization
- Dependencies installation
- File structure setup
- API endpoint design
- Database integration
- Error handling
- Testing strategy

### Step 3: Set Up Package.json

Type:
```
edit package.json "Create a new package.json for a Node.js Express API with task management features"
```

Expected file creation:
```json
{
  "name": "fennec-todo-api",
  "version": "1.0.0",
  "description": "Task management REST API built with Fennec",
  "main": "server.js",
  "scripts": {
    "start": "node server.js",
    "dev": "nodemon server.js",
    "test": "jest"
  },
  "dependencies": {
    "express": "^4.18.2",
    "cors": "^2.8.5",
    "helmet": "^7.0.0",
    "express-rate-limit": "^6.8.1"
  },
  "devDependencies": {
    "nodemon": "^3.0.1",
    "jest": "^29.6.2",
    "supertest": "^6.3.3"
  }
}
```

### Step 4: Install Dependencies

Type:
```
run "npm install"
```

Fennec will execute the npm install command and show you the installation progress.

### Step 5: Create the Server File

Type:
```
edit server.js "Create an Express.js server with CORS, helmet security, rate limiting, and basic task CRUD routes"
```

Expected file creation:
```javascript
const express = require('express');
const cors = require('cors');
const helmet = require('helmet');
const rateLimit = require('express-rate-limit');

const app = express();
const PORT = process.env.PORT || 3000;

// Middleware
app.use(helmet());
app.use(cors());
app.use(express.json({ limit: '10mb' }));

// Rate limiting
const limiter = rateLimit({
  windowMs: 15 * 60 * 1000, // 15 minutes
  max: 100 // limit each IP to 100 requests per windowMs
});
app.use(limiter);

// In-memory task storage (replace with database in production)
let tasks = [];
let nextId = 1;

// Routes
app.get('/api/tasks', (req, res) => {
  res.json(tasks);
});

app.post('/api/tasks', (req, res) => {
  const { title, description, priority = 'medium' } = req.body;

  if (!title) {
    return res.status(400).json({ error: 'Title is required' });
  }

  const task = {
    id: nextId++,
    title,
    description,
    priority,
    completed: false,
    createdAt: new Date().toISOString()
  };

  tasks.push(task);
  res.status(201).json(task);
});

app.get('/api/tasks/:id', (req, res) => {
  const id = parseInt(req.params.id);
  const task = tasks.find(t => t.id === id);

  if (!task) {
    return res.status(404).json({ error: 'Task not found' });
  }

  res.json(task);
});

app.put('/api/tasks/:id', (req, res) => {
  const id = parseInt(req.params.id);
  const taskIndex = tasks.findIndex(t => t.id === id);

  if (taskIndex === -1) {
    return res.status(404).json({ error: 'Task not found' });
  }

  const { title, description, priority, completed } = req.body;

  tasks[taskIndex] = {
    ...tasks[taskIndex],
    title: title !== undefined ? title : tasks[taskIndex].title,
    description: description !== undefined ? description : tasks[taskIndex].description,
    priority: priority !== undefined ? priority : tasks[taskIndex].priority,
    completed: completed !== undefined ? completed : tasks[taskIndex].completed,
    updatedAt: new Date().toISOString()
  };

  res.json(tasks[taskIndex]);
});

app.delete('/api/tasks/:id', (req, res) => {
  const id = parseInt(req.params.id);
  const taskIndex = tasks.findIndex(t => t.id === id);

  if (taskIndex === -1) {
    return res.status(404).json({ error: 'Task not found' });
  }

  tasks.splice(taskIndex, 1);
  res.status(204).send();
});

// Error handling middleware
app.use((err, req, res, next) => {
  console.error(err.stack);
  res.status(500).json({ error: 'Something went wrong!' });
});

// 404 handler
app.use('*', (req, res) => {
  res.status(404).json({ error: 'Route not found' });
});

app.listen(PORT, () => {
  console.log(`Server running on port ${PORT}`);
});

module.exports = app;
```

### Step 6: Add Tests

Type:
```
edit tests/api.test.js "Create comprehensive Jest tests for all task API endpoints including error cases"
```

### Step 7: Test the API

Type:
```
run "npm run dev"
```

In another terminal session, type:
```
run "curl http://localhost:3000/api/tasks"
```

### Step 8: Add Documentation

Type:
```
edit README.md "Create API documentation with endpoint descriptions, examples, and setup instructions"
```

### Step 9: Summarize Your Work

Type:
```
summarize --type feature --output memory
```

This creates a comprehensive summary of the API you built and saves it to your project's memory files.

**What you've accomplished:**
- ✅ Built a complete REST API from scratch
- ✅ Implemented CRUD operations for tasks
- ✅ Added security middleware (helmet, rate limiting)
- ✅ Created comprehensive tests
- ✅ Documented the API
- ✅ Learned to use Fennec for full project development

## Tutorial 3: Bug Hunting and Fixing

**Goal**: Learn to identify, analyze, and fix bugs using Fennec's debugging capabilities.

**Time**: 30 minutes

### Prerequisites
- A project with existing code that has bugs
- Fennec configured in read-only mode initially

### Sample Buggy Code

Create this sample file for the tutorial:

```javascript
// buggy-calculator.js
class Calculator {
  constructor() {
    this.history = [];
  }

  add(a, b) {
    const result = a + b;
    this.history.push(`${a} + ${b} = ${result}`);
    return result;
  }

  divide(a, b) {
    const result = a / b; // Bug: No division by zero check
    this.history.push(`${a} / ${b} = ${result}`);
    return result;
  }

  getHistory() {
    return this.history.join('\n');
  }

  average(numbers) {
    let sum = 0;
    for (let i = 0; i <= numbers.length; i++) { // Bug: Off-by-one error
      sum += numbers[i];
    }
    return sum / numbers.length;
  }

  factorial(n) {
    if (n <= 1) return 1;
    return n * this.factorial(n - 1); // Bug: Stack overflow for large numbers
  }
}

module.exports = Calculator;
```

### Step 1: Start Bug Analysis

```bash
fennec --sandbox read-only
```

Type:
```
plan "Analyze the calculator.js file for potential bugs and security issues"
```

**Expected Analysis:**
```
Bug Analysis Plan: calculator.js

1. Code Review Focus Areas
   - Input validation and edge cases
   - Error handling implementation
   - Mathematical operations safety
   - Memory management concerns

2. Identified Potential Issues
   - Division by zero vulnerability
   - Array bounds checking
   - Recursive function limits
   - Input type validation

3. Testing Strategy
   - Unit tests for edge cases
   - Error condition testing
   - Performance testing for large inputs
   - Type checking validation
```

### Step 2: Detailed Bug Analysis

Type:
```
edit buggy-calculator.js "Analyze this code for bugs and potential runtime errors, focusing on the divide, average, and factorial methods"
```

**Expected Bug Report:**
```
Bug Analysis for buggy-calculator.js:

CRITICAL BUGS IDENTIFIED:

1. Division by Zero (divide method, line 12)
   - Impact: Runtime error, application crash
   - Fix: Add zero divisor check
   - Risk: HIGH

2. Off-by-one Error (average method, line 21)
   - Impact: Array index out of bounds, undefined addition
   - Fix: Change <= to < in loop condition
   - Risk: HIGH

3. Stack Overflow Risk (factorial method, line 28)
   - Impact: Call stack exceeded for large numbers
   - Fix: Add input validation and iterative implementation
   - Risk: MEDIUM

4. Missing Input Validation
   - Impact: Type errors, unexpected behavior
   - Fix: Add parameter type checking
   - Risk: MEDIUM
```

### Step 3: Switch to Write Mode and Fix Bugs

Exit read-only mode and restart:
```bash
fennec --sandbox workspace-write
```

### Step 4: Fix Division by Zero

Type:
```
edit buggy-calculator.js "Fix the divide method to handle division by zero with proper error handling"
```

**Expected Fix:**
```javascript
divide(a, b) {
  if (b === 0) {
    throw new Error('Division by zero is not allowed');
  }
  const result = a / b;
  this.history.push(`${a} / ${b} = ${result}`);
  return result;
}
```

### Step 5: Fix Off-by-One Error

Type:
```
edit buggy-calculator.js "Fix the average method to prevent array index out of bounds error"
```

**Expected Fix:**
```javascript
average(numbers) {
  if (!Array.isArray(numbers) || numbers.length === 0) {
    throw new Error('Input must be a non-empty array');
  }

  let sum = 0;
  for (let i = 0; i < numbers.length; i++) { // Fixed: changed <= to <
    if (typeof numbers[i] !== 'number') {
      throw new Error('All elements must be numbers');
    }
    sum += numbers[i];
  }
  return sum / numbers.length;
}
```

### Step 6: Fix Factorial Stack Overflow

Type:
```
edit buggy-calculator.js "Fix the factorial method to prevent stack overflow and handle edge cases"
```

**Expected Fix:**
```javascript
factorial(n) {
  if (typeof n !== 'number' || n < 0 || !Number.isInteger(n)) {
    throw new Error('Input must be a non-negative integer');
  }

  if (n > 170) { // JavaScript number precision limit
    throw new Error('Input too large, would cause overflow');
  }

  // Iterative implementation to prevent stack overflow
  let result = 1;
  for (let i = 2; i <= n; i++) {
    result *= i;
  }
  return result;
}
```

### Step 7: Create Tests for Bug Fixes

Type:
```
edit calculator.test.js "Create comprehensive tests that verify all bug fixes work correctly, including edge cases"
```

### Step 8: Test Your Fixes

Type:
```
run "node -e 'const Calculator = require(\"./buggy-calculator.js\"); const calc = new Calculator(); console.log(\"Testing fixes...\"); try { calc.divide(10, 0); } catch(e) { console.log(\"✅ Division by zero handled:\", e.message); } console.log(\"✅ Average test:\", calc.average([1,2,3,4,5])); console.log(\"✅ Factorial test:\", calc.factorial(5));'"
```

### Step 9: Document the Bug Fixes

Type:
```
summarize --type bugfix --depth detailed
```

**What you've learned:**
- ✅ How to systematically analyze code for bugs
- ✅ Using read-only mode for safe code analysis
- ✅ Identifying different types of bugs and their severity
- ✅ Implementing proper error handling and input validation
- ✅ Creating tests to verify bug fixes
- ✅ Documenting bug fixes for future reference

## Tutorial 4: Code Refactoring

**Goal**: Learn to refactor code systematically for better maintainability and performance.

**Time**: 40 minutes

### Prerequisites
- A project with legacy code that needs refactoring

### Sample Legacy Code

Create this legacy code to refactor:

```javascript
// legacy-user-service.js
var UserService = function() {
  this.users = [];
  this.nextId = 1;
};

UserService.prototype.createUser = function(userData) {
  var user = {
    id: this.nextId++,
    name: userData.name,
    email: userData.email,
    age: userData.age,
    createdAt: new Date()
  };

  // Validation (poor implementation)
  if (!user.name || user.name.length < 2) {
    return { error: "Name is too short" };
  }
  if (!user.email || user.email.indexOf("@") === -1) {
    return { error: "Invalid email" };
  }
  if (user.age < 18) {
    return { error: "User must be 18 or older" };
  }

  // Check for duplicate email
  for (var i = 0; i < this.users.length; i++) {
    if (this.users[i].email === user.email) {
      return { error: "Email already exists" };
    }
  }

  this.users.push(user);
  return user;
};

UserService.prototype.findUserById = function(id) {
  for (var i = 0; i < this.users.length; i++) {
    if (this.users[i].id == id) {
      return this.users[i];
    }
  }
  return null;
};

UserService.prototype.updateUser = function(id, updates) {
  var user = this.findUserById(id);
  if (!user) {
    return { error: "User not found" };
  }

  if (updates.name) user.name = updates.name;
  if (updates.email) user.email = updates.email;
  if (updates.age) user.age = updates.age;
  user.updatedAt = new Date();

  return user;
};

UserService.prototype.deleteUser = function(id) {
  for (var i = 0; i < this.users.length; i++) {
    if (this.users[i].id == id) {
      this.users.splice(i, 1);
      return true;
    }
  }
  return false;
};

module.exports = UserService;
```

### Step 1: Analyze Current Code

Type:
```
plan "Refactor the legacy UserService class to use modern JavaScript features, improve error handling, and enhance maintainability"
```

**Expected Refactoring Plan:**
```
Refactoring Plan: UserService Modernization

1. Language Modernization
   - Convert to ES6+ class syntax
   - Use const/let instead of var
   - Implement proper error handling with custom exceptions
   - Add comprehensive input validation

2. Code Quality Improvements
   - Extract validation logic into separate methods
   - Implement proper email validation with regex
   - Add JSDoc documentation
   - Use Map for O(1) email lookups

3. Architecture Improvements
   - Separate concerns (validation, storage, business logic)
   - Add proper TypeScript support
   - Implement repository pattern
   - Add logging and monitoring hooks

4. Testing and Documentation
   - Create comprehensive unit tests
   - Add integration tests
   - Update documentation
   - Add usage examples
```

### Step 2: Convert to Modern Class Syntax

Type:
```
edit legacy-user-service.js "Convert the UserService from prototype-based to modern ES6 class syntax with proper error handling"
```

**Expected Modern Class:**
```javascript
class UserService {
  constructor() {
    this.users = new Map(); // Use Map for better performance
    this.emailIndex = new Set(); // Track emails for O(1) lookup
    this.nextId = 1;
  }

  /**
   * Creates a new user with validation
   * @param {Object} userData - User data object
   * @param {string} userData.name - User's name
   * @param {string} userData.email - User's email
   * @param {number} userData.age - User's age
   * @returns {Object} Created user object
   * @throws {ValidationError} When validation fails
   */
  createUser(userData) {
    const validationResult = this._validateUserData(userData);
    if (!validationResult.isValid) {
      throw new ValidationError(validationResult.errors);
    }

    if (this.emailIndex.has(userData.email.toLowerCase())) {
      throw new DuplicateEmailError('Email already exists');
    }

    const user = {
      id: this.nextId++,
      name: userData.name.trim(),
      email: userData.email.toLowerCase(),
      age: userData.age,
      createdAt: new Date(),
      updatedAt: null
    };

    this.users.set(user.id, user);
    this.emailIndex.add(user.email);

    return user;
  }

  /**
   * Validates user data
   * @private
   */
  _validateUserData(userData) {
    const errors = [];

    if (!userData || typeof userData !== 'object') {
      errors.push('User data must be an object');
      return { isValid: false, errors };
    }

    // Name validation
    if (!userData.name || typeof userData.name !== 'string' || userData.name.trim().length < 2) {
      errors.push('Name must be at least 2 characters long');
    }

    // Email validation
    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    if (!userData.email || typeof userData.email !== 'string' || !emailRegex.test(userData.email)) {
      errors.push('Invalid email format');
    }

    // Age validation
    if (typeof userData.age !== 'number' || userData.age < 18 || userData.age > 120) {
      errors.push('Age must be between 18 and 120');
    }

    return { isValid: errors.length === 0, errors };
  }

  findUserById(id) {
    if (typeof id !== 'number' || id <= 0) {
      throw new ValidationError('ID must be a positive number');
    }

    return this.users.get(id) || null;
  }

  updateUser(id, updates) {
    const user = this.findUserById(id);
    if (!user) {
      throw new NotFoundError('User not found');
    }

    const validationResult = this._validatePartialUserData(updates);
    if (!validationResult.isValid) {
      throw new ValidationError(validationResult.errors);
    }

    // Check email uniqueness if email is being updated
    if (updates.email && updates.email.toLowerCase() !== user.email) {
      if (this.emailIndex.has(updates.email.toLowerCase())) {
        throw new DuplicateEmailError('Email already exists');
      }

      this.emailIndex.delete(user.email);
      this.emailIndex.add(updates.email.toLowerCase());
    }

    // Update user properties
    Object.assign(user, {
      ...updates,
      updatedAt: new Date()
    });

    return user;
  }

  deleteUser(id) {
    const user = this.findUserById(id);
    if (!user) {
      return false;
    }

    this.users.delete(id);
    this.emailIndex.delete(user.email);
    return true;
  }

  getAllUsers() {
    return Array.from(this.users.values());
  }

  getUserCount() {
    return this.users.size;
  }
}

// Custom Error Classes
class ValidationError extends Error {
  constructor(errors) {
    super(Array.isArray(errors) ? errors.join(', ') : errors);
    this.name = 'ValidationError';
    this.errors = Array.isArray(errors) ? errors : [errors];
  }
}

class DuplicateEmailError extends Error {
  constructor(message) {
    super(message);
    this.name = 'DuplicateEmailError';
  }
}

class NotFoundError extends Error {
  constructor(message) {
    super(message);
    this.name = 'NotFoundError';
  }
}

module.exports = { UserService, ValidationError, DuplicateEmailError, NotFoundError };
```

### Step 3: Add TypeScript Support

Type:
```
edit user-service.d.ts "Create TypeScript definitions for the refactored UserService"
```

### Step 4: Create Comprehensive Tests

Type:
```
edit user-service.test.js "Create comprehensive unit tests for the refactored UserService with all edge cases and error scenarios"
```

### Step 5: Performance Comparison

Type:
```
edit performance-test.js "Create a performance comparison test between the old and new UserService implementations"
```

Type:
```
run "node performance-test.js"
```

### Step 6: View the Refactoring Changes

Type:
```
diff legacy-user-service.js
```

### Step 7: Document the Refactoring

Type:
```
summarize --type refactoring --depth comprehensive --output file
```

**What you've accomplished:**
- ✅ Converted legacy prototype-based code to modern ES6 classes
- ✅ Improved performance with better data structures (Map vs Array)
- ✅ Added comprehensive input validation and error handling
- ✅ Created custom error classes for better error categorization
- ✅ Added TypeScript support for better development experience
- ✅ Created comprehensive tests
- ✅ Measured performance improvements
- ✅ Documented the entire refactoring process

## Tutorial 5: Security Review and Hardening

**Goal**: Learn to identify security vulnerabilities and implement security best practices.

**Time**: 35 minutes

### Prerequisites
- A web application with potential security issues
- Fennec in read-only mode initially

### Sample Vulnerable Code

Create this vulnerable authentication system:

```javascript
// vulnerable-auth.js
const express = require('express');
const crypto = require('crypto');
const app = express();

app.use(express.json());

// Vulnerable: Hardcoded secret
const JWT_SECRET = 'secret123';

// Vulnerable: Plaintext password storage
let users = [
  { id: 1, username: 'admin', password: 'admin123', role: 'admin' },
  { id: 2, username: 'user', password: 'password', role: 'user' }
];

let sessions = {};

// Vulnerable: No rate limiting, SQL injection potential
app.post('/login', (req, res) => {
  const { username, password } = req.body;

  // Vulnerable: No input validation
  if (!username || !password) {
    return res.status(400).json({ error: 'Missing credentials' });
  }

  // Vulnerable: Direct comparison, timing attack potential
  const user = users.find(u => u.username === username && u.password === password);

  if (!user) {
    return res.status(401).json({ error: 'Invalid credentials' });
  }

  // Vulnerable: Weak session token generation
  const sessionToken = Math.random().toString(36);
  sessions[sessionToken] = { userId: user.id, username: user.username };

  // Vulnerable: Sensitive data in response
  res.json({
    token: sessionToken,
    user: user // Exposes password!
  });
});

// Vulnerable: No authentication middleware
app.get('/admin', (req, res) => {
  const token = req.headers.authorization;

  if (!sessions[token]) {
    return res.status(401).json({ error: 'Unauthorized' });
  }

  // Vulnerable: No authorization check
  res.json({ message: 'Admin area', secret: 'top-secret-data' });
});

// Vulnerable: No HTTPS enforcement, no security headers
app.listen(3000, () => {
  console.log('Server running on port 3000');
});
```

### Step 1: Security Analysis

```bash
fennec --sandbox read-only
```

Type:
```
plan "Perform a comprehensive security analysis of the vulnerable authentication system and identify all security issues"
```

**Expected Security Report:**
```
Security Analysis Report: Authentication System

CRITICAL VULNERABILITIES:

1. Hardcoded Secrets (Line 6)
   - Risk: HIGH - JWT secret exposed in code
   - Impact: Token forging, complete auth bypass
   - Fix: Use environment variables

2. Plaintext Password Storage (Line 9)
   - Risk: CRITICAL - Passwords stored in plain text
   - Impact: Complete user compromise
   - Fix: Use bcrypt hashing with salt

3. Weak Session Token Generation (Line 28)
   - Risk: HIGH - Predictable session tokens
   - Impact: Session hijacking
   - Fix: Use cryptographically secure random generation

4. Information Disclosure (Line 33)
   - Risk: HIGH - Password exposed in login response
   - Impact: Credential exposure
   - Fix: Remove sensitive data from responses

5. Missing Authorization (Line 38)
   - Risk: HIGH - No role-based access control
   - Impact: Privilege escalation
   - Fix: Implement proper authorization middleware

HIGH-RISK ISSUES:

6. No Rate Limiting
   - Risk: Brute force attacks
   - Fix: Implement rate limiting

7. Missing Input Validation
   - Risk: Injection attacks
   - Fix: Comprehensive input sanitization

8. No Security Headers
   - Risk: Various client-side attacks
   - Fix: Add helmet middleware

9. Timing Attacks in Authentication
   - Risk: Username enumeration
   - Fix: Constant-time comparisons
```

### Step 2: Exit Read-Only Mode

```bash
fennec --sandbox workspace-write --ask-for-approval
```

### Step 3: Fix Hardcoded Secrets

Type:
```
edit vulnerable-auth.js "Replace hardcoded JWT secret with environment variable and add proper secret management"
```

### Step 4: Implement Secure Password Hashing

Type:
```
edit vulnerable-auth.js "Replace plaintext password storage with bcrypt hashing and secure password verification"
```

### Step 5: Fix Session Management

Type:
```
edit vulnerable-auth.js "Implement secure session token generation using crypto.randomBytes and add session expiration"
```

### Step 6: Add Security Middleware

Type:
```
edit vulnerable-auth.js "Add helmet for security headers, rate limiting, and input validation middleware"
```

### Step 7: Implement Authorization

Type:
```
edit vulnerable-auth.js "Add proper authentication and authorization middleware with role-based access control"
```

### Step 8: Create Secure Version

The final secure version should look like:

```javascript
// secure-auth.js
const express = require('express');
const bcrypt = require('bcrypt');
const crypto = require('crypto');
const rateLimit = require('express-rate-limit');
const helmet = require('helmet');
const validator = require('validator');

const app = express();

// Security middleware
app.use(helmet());
app.use(express.json({ limit: '1mb' }));

// Rate limiting
const loginLimiter = rateLimit({
  windowMs: 15 * 60 * 1000, // 15 minutes
  max: 5, // limit each IP to 5 requests per windowMs
  message: 'Too many login attempts, try again later',
  standardHeaders: true,
  legacyHeaders: false,
});

// Secure configuration
const JWT_SECRET = process.env.JWT_SECRET || crypto.randomBytes(32).toString('hex');
const BCRYPT_ROUNDS = 12;
const SESSION_TIMEOUT = 30 * 60 * 1000; // 30 minutes

// Secure password hashes (pre-generated with bcrypt)
let users = [
  {
    id: 1,
    username: 'admin',
    passwordHash: '$2b$12$...' // bcrypt hash of 'admin123'
    role: 'admin'
  },
  {
    id: 2,
    username: 'user',
    passwordHash: '$2b$12$...' // bcrypt hash of 'password'
    role: 'user'
  }
];

let sessions = new Map();

// Authentication middleware
const authenticateToken = (req, res, next) => {
  const authHeader = req.headers['authorization'];
  const token = authHeader && authHeader.split(' ')[1];

  if (!token) {
    return res.status(401).json({ error: 'Access token required' });
  }

  const session = sessions.get(token);
  if (!session || session.expiresAt < Date.now()) {
    sessions.delete(token);
    return res.status(403).json({ error: 'Token expired or invalid' });
  }

  req.user = session.user;
  next();
};

// Authorization middleware
const requireRole = (role) => {
  return (req, res, next) => {
    if (req.user.role !== role) {
      return res.status(403).json({ error: 'Insufficient permissions' });
    }
    next();
  };
};

// Input validation
const validateLoginInput = (req, res, next) => {
  const { username, password } = req.body;

  if (!username || !password) {
    return res.status(400).json({ error: 'Username and password required' });
  }

  if (typeof username !== 'string' || typeof password !== 'string') {
    return res.status(400).json({ error: 'Invalid input types' });
  }

  if (username.length > 50 || password.length > 128) {
    return res.status(400).json({ error: 'Input too long' });
  }

  if (!validator.isAlphanumeric(username)) {
    return res.status(400).json({ error: 'Invalid username format' });
  }

  next();
};

// Secure login endpoint
app.post('/login', loginLimiter, validateLoginInput, async (req, res) => {
  try {
    const { username, password } = req.body;

    // Find user
    const user = users.find(u => u.username === username);

    // Constant-time comparison to prevent timing attacks
    let isValid = false;
    if (user) {
      isValid = await bcrypt.compare(password, user.passwordHash);
    } else {
      // Perform dummy bcrypt to prevent timing attacks
      await bcrypt.compare(password, '$2b$12$dummy.hash.to.prevent.timing.attacks');
    }

    if (!isValid) {
      return res.status(401).json({ error: 'Invalid credentials' });
    }

    // Generate secure session token
    const sessionToken = crypto.randomBytes(32).toString('hex');
    const expiresAt = Date.now() + SESSION_TIMEOUT;

    sessions.set(sessionToken, {
      user: {
        id: user.id,
        username: user.username,
        role: user.role
      },
      expiresAt
    });

    // Clean response (no sensitive data)
    res.json({
      token: sessionToken,
      expiresAt,
      user: {
        id: user.id,
        username: user.username,
        role: user.role
      }
    });

  } catch (error) {
    console.error('Login error:', error);
    res.status(500).json({ error: 'Internal server error' });
  }
});

// Protected admin endpoint
app.get('/admin', authenticateToken, requireRole('admin'), (req, res) => {
  res.json({
    message: 'Admin area',
    user: req.user.username
  });
});

// Logout endpoint
app.post('/logout', authenticateToken, (req, res) => {
  const token = req.headers['authorization'].split(' ')[1];
  sessions.delete(token);
  res.json({ message: 'Logged out successfully' });
});

// Clean up expired sessions periodically
setInterval(() => {
  const now = Date.now();
  for (const [token, session] of sessions.entries()) {
    if (session.expiresAt < now) {
      sessions.delete(token);
    }
  }
}, 5 * 60 * 1000); // Clean up every 5 minutes

app.listen(3000, () => {
  console.log('Secure server running on port 3000');
});
```

### Step 9: Security Testing

Type:
```
edit security-tests.js "Create comprehensive security tests to verify all vulnerabilities are fixed"
```

Type:
```
run "npm test security-tests.js"
```

### Step 10: Security Documentation

Type:
```
summarize --type security --depth comprehensive
```

**What you've learned:**
- ✅ How to identify common web application security vulnerabilities
- ✅ Implementing secure password hashing with bcrypt
- ✅ Secure session management and token generation
- ✅ Input validation and sanitization techniques
- ✅ Rate limiting and DoS prevention
- ✅ Authorization and role-based access control
- ✅ Security headers and middleware implementation
- ✅ Security testing and vulnerability verification

## Tutorial 6: Advanced Memory Management

**Goal**: Master Fennec's memory system for optimal context management and project continuity.

**Time**: 25 minutes

### Prerequisites
- A project with existing development history
- Multiple development sessions to track

### Step 1: Initialize Memory System

```bash
# Start Fennec in your project
fennec
```

Type:
```
plan "Set up comprehensive project memory management including AGENTS.md, project brief, and development context tracking"
```

### Step 2: Create AGENTS.md

Type:
```
edit AGENTS.md "Create comprehensive project guidelines and coding standards for a TypeScript React application with Node.js backend"
```

**Expected AGENTS.md:**
```markdown
# Project Guidelines: Full-Stack TypeScript Application

## Project Overview
E-commerce platform with React frontend and Node.js backend, using TypeScript throughout.

## Technology Stack
- **Frontend**: React 18, TypeScript, TailwindCSS, Zustand
- **Backend**: Node.js, Express, TypeScript, Prisma, PostgreSQL
- **Testing**: Jest, React Testing Library, Supertest
- **DevOps**: Docker, GitHub Actions, AWS

## Coding Standards

### TypeScript
- Use strict TypeScript configuration
- Define interfaces for all data structures
- Use generic types where appropriate
- Avoid `any` type, use `unknown` instead

### React
- Use functional components with hooks
- Implement proper error boundaries
- Use custom hooks for complex logic
- Follow the established component structure

### Backend
- Use dependency injection pattern
- Implement proper error handling middleware
- Use DTOs for API data validation
- Follow REST API conventions

### Testing
- Maintain minimum 80% code coverage
- Write integration tests for API endpoints
- Use React Testing Library best practices
- Mock external dependencies

## Security Requirements
- Validate all input data with Joi
- Use bcrypt for password hashing
- Implement JWT with refresh tokens
- Add rate limiting to all public endpoints
- Use HTTPS in production

## Development Workflow
1. Create feature branch from main
2. Write tests first (TDD approach)
3. Implement feature with documentation
4. Run all tests and linting
5. Create PR with detailed description
6. Require code review approval
7. Merge to main after CI passes

## Code Review Checklist
- [ ] Code follows TypeScript and ESLint rules
- [ ] All tests pass and coverage maintained
- [ ] No hardcoded secrets or credentials
- [ ] Proper error handling implemented
- [ ] Documentation updated
- [ ] Performance impact considered

## Architecture Decisions
- **State Management**: Zustand for global state, React state for local
- **Styling**: TailwindCSS with component-based approach
- **API Design**: RESTful with consistent error responses
- **Database**: PostgreSQL with Prisma ORM
- **Authentication**: JWT with httpOnly cookies
- **File Structure**: Domain-driven design principles
```

### Step 3: Create Project Brief

Type:
```
edit projectbrief.md "Create comprehensive project brief documenting the e-commerce platform's goals, current status, and roadmap"
```

### Step 4: Set Up Active Context

Type:
```
edit activeContext.md "Document current development sprint focusing on user authentication and product catalog features"
```

### Step 5: Initialize Progress Tracking

Type:
```
summarize --type project-init --output memory
```

This creates the initial progress.md file with your current session summary.

### Step 6: Simulate Development Session

Type:
```
plan "Implement user authentication system with registration, login, and password reset functionality"
```

Type:
```
edit src/auth/AuthService.ts "Create authentication service with registration, login, logout, and token refresh methods"
```

Type:
```
run "npm test auth"
```

### Step 7: Update Memory with Progress

Type:
```
summarize --type progress --depth detailed --output memory
```

### Step 8: Search Memory Context

Type:
```
plan "Add password reset functionality to the authentication system"
```

Notice how Fennec automatically includes context from your memory files about authentication requirements and security standards.

### Step 9: Advanced Memory Queries

Create a complex development scenario:

Type:
```
edit src/products/ProductService.ts "Implement product search with filtering and pagination"
```

Type:
```
summarize --type feature --output memory
```

Now test memory retrieval:

Type:
```
plan "Optimize the product search performance for large catalogs"
```

Fennec should include context about:
- Previous product service implementation
- Performance requirements from AGENTS.md
- Database optimization patterns from project history

### Step 10: Memory Management Commands

Type:
```
# View memory stats
status memory

# Search memory for specific topics
search "authentication security"

# Update active context
edit activeContext.md "Shift focus to product catalog optimization and performance tuning"

# Create milestone summary
summarize --type milestone --depth comprehensive --output both
```

### Step 11: Cross-Session Continuity

Exit Fennec and restart to test memory persistence:

```bash
fennec
```

Type:
```
plan "Continue work on product search optimization based on previous session"
```

Verify that Fennec loads and uses your memory context appropriately.

### Step 12: Memory Optimization

Type:
```
# Clean up old memory entries
summarize --type cleanup --consolidate-history

# Update project brief with current status
edit projectbrief.md "Update project status to reflect completed authentication system and current product catalog work"
```

**What you've mastered:**
- ✅ Setting up comprehensive project memory with AGENTS.md
- ✅ Creating and maintaining project briefs and context files
- ✅ Using memory for cross-session continuity
- ✅ Automatic context retrieval and injection
- ✅ Memory search and query capabilities
- ✅ Progress tracking and milestone documentation
- ✅ Memory optimization and cleanup
- ✅ Advanced memory management workflows

## Next Steps

After completing these tutorials, you should be proficient with:

1. **Basic Fennec Operations**: Planning, editing, running commands, and summarizing
2. **Project Development**: Building complete applications from scratch
3. **Debugging and Bug Fixing**: Systematic analysis and resolution of issues
4. **Code Refactoring**: Modernizing and improving existing codebases
5. **Security Analysis**: Identifying and fixing security vulnerabilities
6. **Memory Management**: Optimizing context and maintaining project continuity

### Advanced Topics to Explore

- **Custom Command Development**: Creating your own Fennec commands
- **Provider Integration**: Setting up additional LLM providers
- **CI/CD Integration**: Using Fennec in automated workflows
- **Team Collaboration**: Sharing memory files and project configurations
- **Performance Optimization**: Tuning Fennec for large projects
- **Security Hardening**: Advanced security configurations and auditing

### Community Resources

- **Documentation**: Explore the complete documentation in `docs/`
- **Examples**: Check out more examples in `examples/`
- **Issues and Support**: GitHub Issues for bug reports and feature requests
- **Discussions**: GitHub Discussions for questions and community interaction

Happy coding with Fennec! 🦊