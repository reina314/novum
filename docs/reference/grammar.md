---
layout: page
title: Grammar-Oriented Summary
parent: Language Reference
---

# Grammar-Oriented Summary

This page summarizes the current surface grammar rather than presenting a formal parser grammar.

```text
program        := expression*
expression     := assignment | control | declaration | postfix-expression | ...
assignment     := logical-expression ('=' | compound-assignment) assignment
call           := expression '(' argument-list? ')'
field          := expression '.' identifier
index          := expression '[' index-expression ']'
argument       := identifier '=' expression | expression
import         := 'import' module-path ('as' identifier)?
```

Compound assignment operators include the arithmetic forms currently supported by the parser, such as `+=`, `-=`, `*=`, `/=`, and `%=`.

Type declarations have the form:

```text
struct Name { member* }
class Name  { member* }

member := identifier
        | identifier '=' expression
```

Inside a type declaration, a lambda value assigned to a member is a method; a non-lambda expression assigned to a member is a field default.

This page is intentionally descriptive rather than a substitute for the parser source when exact token-level behavior matters.

