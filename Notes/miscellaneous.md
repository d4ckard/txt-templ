# Miscellaneous

- The expressions *name* and *identifier* are sometimes used interchangeably
and both refer to what is called an [element's *identifier*](spec.md#identifiers) in the specification.

## EBNF grammar

This is a rough idea of the grammar accepted by the parser. In practice implementation
details might vary (E.g. the implementation will accept many more characters than the
`<char>` rule in this grammar does).



```ebnf
<template>    ::= <locale>? <element>+
<locale>      ::= "locale" <whitespaces> ":" <whitespaces> /* a valid locale value (managed externally) */
<element>     ::= <text> | <key> | <option> | <constant>
<text>        ::= (<chars> | <whitespace> | [0-9])+
<key>         ::= "{" <ident> <default>? "}"
<option>      ::= "${" <ident> <default>? "}" 
<constant>    ::= "$" <ident>
<default>     ::= ":" <element>
<ident>       ::= (<char> | [0-9])+
<char>        ::= ([A-Z] | [a-z])
<chars>       ::= <char>+
<whitespace>  ::= (" " | "\t" | "\n")
<whitespaces> ::= <whitespace>+
```
