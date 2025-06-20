# Task Management Directive

## Overview
This document defines the standard process for creating, updating, and validating tasks in the Praxis Ai project. All tasks must follow these guidelines to ensure consistency, traceability, and quality.

## Task Naming Convention

Tasks follow a hierarchical numeric pattern:
```
TASK-XXXX-YY-ZZ
```
- **XXXX**: 4-digit task ID (0001-9999)
- **YY**: 2-digit sub-task ID (00 for parent, 01-99 for sub-tasks)
- **ZZ**: 2-digit version/revision number (starting at 00)

Examples:
- `TASK-0001-00-00`: Parent task
- `TASK-0001-01-00`: First sub-task of task 0001
- `TASK-0001-01-01`: First revision of sub-task

## Task Structure

### 1. Task Header
```markdown
# TASK-XXXX-YY-ZZ: [Brief Title]

**Status**: [ ] Not Started | [ ] In Progress | [ ] Blocked | [ ] Complete | [ ] Abandoned
**Created**: YYYY-MM-DD
**Updated**: YYYY-MM-DD
**Assignee**: [Name/Handle]
**Priority**: P0 (Critical) | P1 (High) | P2 (Medium) | P3 (Low)
**Parent Task**: TASK-XXXX-00-00 (if applicable)
**Dependencies**: List of TASK-XXXX-YY-ZZ
**Estimated Effort**: XS (1h) | S (4h) | M (1d) | L (3d) | XL (1w+)
```

### 2. User Story
```markdown
## User Story
As a [type of user],
I want [an action or feature],
So that [benefit/value].
```

### 3. Context Gathering
```markdown
## Context & Research

### Current State Analysis
- [ ] Review existing codebase in relevant directories
- [ ] Document current functionality
- [ ] Identify integration points
- [ ] Note technical constraints

### API Documentation Review
- [ ] Latest API version: [version]
- [ ] Relevant endpoints: [list]
- [ ] Breaking changes: [if any]
- [ ] New features available: [list]

### Technical Research
- [ ] Similar implementations reviewed
- [ ] Best practices identified
- [ ] Performance considerations noted
- [ ] Security implications assessed
```

### 4. Acceptance Criteria
```markdown
## Acceptance Criteria

### Functional Requirements
- [ ] [Specific, measurable requirement]
- [ ] [Another requirement]
- [ ] Error handling for [specific scenarios]
- [ ] Performance: [specific metrics]

### Non-Functional Requirements
- [ ] Code follows project style guide
- [ ] Documentation updated
- [ ] Tests achieve >80% coverage
- [ ] No security vulnerabilities introduced
```

### 5. Behavioral Specifications (Gherkin)
```gherkin
## Behavioral Specifications

Feature: [Feature name]
  As a [user type]
  I want [feature]
  So that [benefit]

  Background:
    Given [initial context]
    And [additional context]

  Scenario: [Happy path scenario]
    Given [initial state]
    When [action taken]
    Then [expected outcome]
    And [additional outcome]

  Scenario: [Error scenario]
    Given [initial state]
    When [error condition]
    Then [error handling]
    And [recovery action]

  Scenario Outline: [Parameterized scenario]
    Given [state with <parameter>]
    When [action with <input>]
    Then [outcome should be <output>]

    Examples:
      | parameter | input | output |
      | value1    | data1 | result1 |
      | value2    | data2 | result2 |
```

### 6. Implementation Plan
```markdown
## Implementation Plan

### Phase 1: Setup & Research
1. [ ] Gather requirements from stakeholders
2. [ ] Review existing code and documentation
3. [ ] Set up development environment
4. [ ] Create feature branch: `feature/TASK-XXXX`

### Phase 2: Development
1. [ ] Implement core functionality
2. [ ] Add error handling
3. [ ] Write unit tests
4. [ ] Write integration tests
5. [ ] Update documentation

### Phase 3: Validation
1. [ ] Run all tests locally
2. [ ] Perform manual testing
3. [ ] Code review checklist
4. [ ] Performance testing
5. [ ] Security scan

### Phase 4: Deployment
1. [ ] Create pull request
2. [ ] Address review feedback
3. [ ] Merge to main branch
4. [ ] Deploy to staging
5. [ ] Verify in production
```

### 7. Test Plan
```markdown
## Test Plan

### Unit Tests
- [ ] Component: [name] - Test cases: [list]
- [ ] Function: [name] - Test cases: [list]
- [ ] Edge cases covered

### Integration Tests
- [ ] API endpoint tests
- [ ] Database integration tests
- [ ] External service mocks

### E2E Tests
- [ ] User workflow: [description]
- [ ] Error scenarios
- [ ] Performance benchmarks
```

### 8. Definition of Done
```markdown
## Definition of Done
- [ ] All acceptance criteria met
- [ ] All tests passing
- [ ] Code reviewed and approved
- [ ] Documentation updated
- [ ] No critical or high severity bugs
- [ ] Performance benchmarks met
- [ ] Security scan passed
- [ ] Deployed to production
```

## Task Lifecycle

### Creating a Task
1. Research and gather context
2. Pull latest API documentation
3. Review similar existing tasks
4. Use task template
5. Assign unique TASK-XXXX ID
6. Link parent/dependencies

### Updating a Task
1. Increment revision number (ZZ)
2. Update "Updated" date
3. Document changes in revision history
4. Re-validate dependencies
5. Update status

### Validating a Task
1. Ensure all sections completed
2. Verify Gherkin scenarios executable
3. Confirm test coverage adequate
4. Review against coding standards
5. Check dependency resolution

## Task Status Definitions

- **Not Started**: Task defined but work not begun
- **In Progress**: Active development ongoing
- **Blocked**: Cannot proceed due to dependency/issue
- **Complete**: All acceptance criteria met, deployed
- **Abandoned**: Task cancelled or no longer needed

## Best Practices

1. **One Task, One Purpose**: Each task should have a single, clear objective
2. **Measurable Outcomes**: All criteria must be objectively verifiable
3. **Dependencies First**: Always complete dependencies before starting
4. **Test-Driven**: Write tests before implementation
5. **Document Everything**: Future you will thank present you
6. **Review Early**: Get feedback on task definition before starting

## Task File Organization
```
tasks/
├── tasks-directive.md      # This file
├── implementation-directives.md
├── roadmap.md
├── active/                 # Current sprint tasks
│   ├── TASK-0001-00-00.md
│   └── TASK-0002-00-00.md
├── backlog/               # Future tasks
│   └── TASK-0003-00-00.md
├── completed/             # Archived tasks
│   └── TASK-0000-00-00.md
└── templates/             # Task templates
    └── task-template.md
```

## Validation Checklist

Before marking a task complete:
- [ ] All code is tested
- [ ] Documentation is current
- [ ] Performance verified
- [ ] Security validated
- [ ] Accessibility checked
- [ ] Code reviewed
- [ ] Documentation updated
- [ ] Git commit created with proper message
- [ ] Deployed successfully
- [ ] Monitoring in place

## Task Completion Process

### Documentation Updates
Upon task completion, ensure the following documentation is updated:

1. **CLAUDE.md**: Update current status and add any new features or architectural changes
2. **README.md**: Update if the changes affect user-facing features or installation
3. **Task file**: Add implementation results and validation summary
4. **API documentation**: Update if any public interfaces change

### Git Commit Guidelines

Each completed task should result in a well-structured commit:

#### Commit Message Format
```
feat(scope): brief description of changes

Implements TASK-XXXX-YY-ZZ: [Task Title]

## Changes
- Feature 1: description
- Feature 2: description  
- Fix: description

## Validation
- All tests passing
- Performance requirements met
- Documentation updated

🤖 Generated with [Claude Code](https://claude.ai/code)

Co-Authored-By: Claude <noreply@anthropic.com>
```

#### Commit Types
- `feat`: New feature implementation
- `fix`: Bug fix
- `docs`: Documentation changes
- `refactor`: Code refactoring without feature changes
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

#### Pre-commit Checklist
- [ ] All tests pass (`cargo test`)
- [ ] Code is formatted (`cargo fmt`)
- [ ] Linting passes (`cargo clippy`)
- [ ] Documentation is updated
- [ ] Task status is updated to "Complete"
- [ ] Task file includes implementation results

### Repository Management

#### File Organization
- Move completed task files from `tasks/active/` to `tasks/completed/`
- Ensure `.gitignore` is comprehensive and up-to-date
- Keep repository clean of build artifacts and temporary files

#### Documentation Maintenance
- CLAUDE.md should always reflect current project status
- README.md should be updated for significant milestones
- Task files should include validation results and lessons learned