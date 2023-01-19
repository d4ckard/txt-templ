# EBNF grammar

```bnf
<template>		::= <locale>? <element>+
<locale>      ::= "locale" <whitespaces> ":" <whitespaces> /* a valid locale value (managed externally) */
<element>			::= <text> | <key> | <option> | <constant>
<text>     		::= (<chars> | <whitspace> | [0-9])+
<key>      		::= "{" <ident> <default>? "}"
<option>   		::= "${" <ident> <default>? "}" 
<constant> 		::= "$" <ident>
<default>  		::= ":" <element>
<ident>    		::= (<char> | [0-9])+
<whitspace>		::= (" " | "\t" | "\n")
<whitspaces>	::= <whitspace>+
<char>     		::= ([A-Z] | [a-z])
<chars>    		::= <char>+
```
