# EBNF grammar

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
