# Template specification

A template may contain four different types of elements:

  1. [Text literals](#text-literals)
  2. [Keys](#keys)
  3. [Options](#options)
  4. [Constants](#constants)

Additionally, a template may contain an optional
[locale setting](#locale) at the beginning of the template.


## Text literals
By default any plain text in a template is considered a text literal.
Text literals may contain any valid Unicode character[^1] except for the characters `{`, `}` and `$`[^2].
When filling out a template, text literals are copied directly into the result.

### Example
A valid text literal containing special characters and non-ASCII characters:

```
Hello, my name is Jörg!
```

## Keys

Keys are named variables placed at any position in the template. When filling out the template,
a text literal needs to be specified by the user for each key. This text literal is then
substituted at the key's position in the template.
A valid key is made up of the key's name ([identifier](#identifiers)) contained inside curly braces (`{` and `}`).

A key could be used to insert any name into the same template:

```
Hello, my name is {name}!
```

## Options

Options are also named variables, just like keys. But there are two important differences
between keys and options:

1. An option can not have any arbitrary name. All valid options are
specified in the `UserContentState`. If the name of an option contained in the template
is not found there, the template will not be accepted.

2. When filling out an option in the template, instead of specifying the text literal
to be substituted, one of the option's possible *choices* is selected. Each possible
choice for an option is also specified in the `UserContentState`, where it directs
to a text literal which will be substituted in the template.

A valid option is made up of the option's identifier contained inside curly braces with a `$` symbol
before of the opening brace (`{`).

### Example
An option might be used to select a greeting on the fly:

`UserContentState`:

```
Options:
  - greeting:
    h   -> "Hello"
    w   -> "Wazzz-up"
    dlg -> "Dear Ladies and Gentlemen"

... more options
```
Template:

```
${greeting}, my name is {name}!
```

To fill out this template both a text literal for the key `name` and a choice for
the option `greeting` are required. All possible choices for `greeting` are `h`, `w` and `dlg`.


## Constants

Constants are identifiers for *constant* text literals stored in the `UserContentState`.
For this reason a constant is valid only if its name is found in the `UserContentState`.

A constant is made up of a `$` symbol, followed by the constant's identifier.
The identifier of the constant ends upon encountering any character which may
not be contained inside an identifier.

### Example
Constants are very useful for any kind of text which almost never changes
but is repeated very often (e.g. your name, email, phone number etc.):

`UserContentState`:

```
Options: ...

Constants:
  - Me -> "Benjamin"

... more constants
```
Template:

```
${greeting}, my name is $Me! I am here to tell you this {message}.
```

### Meta constants

Meta constants are constants whose identifiers are pre-defined to translate to some
dynamic value which depends on the context of the compilation.
For example, the meta constant `$now` will be translated into the time of compilation.



## Identifiers

Identifiers are used as the names of variable elements (keys, options and constants).

Stricter rules apply to identifiers than to text literals when it comes to the symbols
they are allowed to contain: identifiers may contain the ASCII symbols A-Z, a-z and 0-9.

### Examples of valid identifiers
`name` (only lowercase)

`Me` (mixed-case)

`MAIL` (only uppercase)

`Address12` (mixed-case combined with digits)

`0275` (only digits)

### Examples of invalid identifiers
`my-name` (contains forbidden special character)

`Straße` (contains forbidden Unicode character)


## Defaults
When using keys and options[^3] you can specify default values which will be used if
no value is explicitly given when filling out the template. This way by specifying
a default considering the element when filling out the template becomes optional.
If, however a value if given for the element, this value will overwrite the default value.

Elements of any type can be used as defaults. Defaults may also be nested, meaning a
default for a key may have a default by itself and so on. If at some point a
text literal is encountered as the default's value[^4], it will be propagated as the default
for all elements in the chain of nested elements.

A default is specified by following up the identifier of the current element
(which is either a key or an option) with a colon (`:`) and then the element which
should be used as the default value. The identifier of the current element,
the colon and the default may not be separated by any whitespace characters.

### Examples
This is a key with another key as a default value which then has a text literal as a default itself:

```
{name:{othername:Paul}}
```

Here an option for an email address defaults to the user's work email address if the user doesn't
explicitly specify another email address:

```
${email:$workemail}
```


## Locale
The optional locale setting at the start of the template enables
language or region specific processing[^5]. If the locale is missing
from the template, `en-US` is used as the default locale.
A locale is considered well-formatted if it is a valid
[Unicode Locale Identifier](https://unicode.org/reports/tr35/tr35.html#Unicode_locale_identifier).

The locale setting is specified using the *keyword* `locale` followed
by a colon `:` and valid locale string. The locale string is terminated by
a new-line character (`\n`).
The colon delimiting the `locale` keyword from the locale string
may have optional whitespace characters on both sides.

### Examples
Slightly different ways of setting a template's locale to German:

```
locale : de-DE
Sehr geehrte Frau {name}, ...
```

```
locale: de_DE
Sehr geehrter Herr {name}, ...
```



[^1]: More specifically text literals may contain any valid [Unicode scalar value](https://www.unicode.org/glossary/#unicode_scalar_value) as text literals are represented as lists of [rust chars](https://doc.rust-lang.org/std/primitive.char.html) internally.

[^2]: This could be very inconvenient at times. Especially the `$` symbol is very hard to avoid using in daily use. This needs improvement (Maybe by using the symbol twice: e.g. `$$` (like makefiles)).

[^3]: It would be convenient if one could specify a default choice for an option too. This is not possible right now. (This might work by implementing choices as constants only visible to the option. Then choosing a default for an option works by selecting this constant!)

[^4]: The default itself may still be of any type, not only of the text literal, but a text literal value must be specified for this element.

[^5]: None if this is implemented at this point but having, it enables adding spell checking etc. later on. Maybe [ltex-ls](https://valentjn.github.io/ltex/index.html) can be used for spell-checking.
