# TASK-XXXX-YY-ZZ: [Brief Title]

**Status**: [ ] Not Started | [ ] In Progress | [ ] Blocked | [ ] Complete | [ ] Abandoned
**Created**: YYYY-MM-DD
**Updated**: YYYY-MM-DD
**Assignee**: [Name/Handle]
**Priority**: P0 (Critical) | P1 (High) | P2 (Medium) | P3 (Low)
**Parent Task**: TASK-XXXX-00-00 (if applicable)
**Dependencies**: List of TASK-XXXX-YY-ZZ
**Estimated Effort**: XS (1h) | S (4h) | M (1d) | L (3d) | XL (1w+)

## User Story
As a [type of user],
I want [an action or feature],
So that [benefit/value].

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

## Behavioral Specifications

```gherkin
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

## Definition of Done
- [ ] All acceptance criteria met
- [ ] All tests passing
- [ ] Code reviewed and approved
- [ ] Documentation updated
- [ ] No critical or high severity bugs
- [ ] Performance benchmarks met
- [ ] Security scan passed
- [ ] Deployed to production