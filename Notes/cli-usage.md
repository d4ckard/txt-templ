# CLI Usage Guide

The purpose of the `txttc` command line tool is to compile the given template.
To do so, it requires both a semi-permanent [**content state**](#content-state)
and the current [**volatile content**](#volatile-content), both of which are
used to substitute the elements in the template.

A path to a template source file is *always* required to be passed through the
`--template` flag:

```bash
# txttc invocation with the minimum amount of arguments.
$ txttc --template my-template.txtt
```

Due to the behavior of the different elements available for use in templates,
their respective content needs to be specified in different places.

## Content state

The *content state* contains the content for **constants** and all available **choices** for all **options**.

If the identifier of a *constant* is encountered when compiling a template, it is looked up in the
*content state*. If a *constant*-entry with this identifier is found, its content will be substituted
in place of the element.

In case an *option's* identifier is found, it's presence in the *content state* will be checked too.

The *content state* does not however contain the actual choice for the *option* (Each *option*
element has a number of associated possible *choices*. They are filled out by selecting a *choice*).
This selection happens in the *volatile content*.

## Volatile Content

The *volatile content* contains all information which is missing from the *content state*.
That is, a single *choice* selection for each *option* and the content for any *key* elements
in the template.

It is called volatile because this content will most likely change with every compilation.
In case it does not change each time one has to compile a template, the entire *volatile
content* can be [saved to file](#persisting-a-draft) too.

## Specifying content state

### The content state file

A file whose contents specify a valid *content state* is called a *content state* file. The
format used for both the *content state* and the *volatile content* file is [YAML](https://yaml.org/).

In this file all know *constants* with their respective contents are presented like this:

```yaml
constants:
  <name of constant>: <content of constant>
  # ... any number of additional constants
```

Because the *content state* file contains all available *choices* for each *option*,
their presentation has one more nesting than the presentation of *constants*:

```yaml
options:
  <name of option>:
    <name of choice>: <content of choice>
    # ... any number of additional choices
  # ... any number of additional options
```

#### Example

A valid *content state* file could look like this:

```yaml
constants:
  # Making a name available for use in template through constants:
  firstName: Paul
  lastName: Atreides

options:
  # Another method of making the same name available through an option:
  name:
    f: Paul
    l: Atreides
    both: Paul Atreides
```

### Methods of selecting a content state file

Currently, there are **three ways** to select a *content state* file when compiling a template.
There is a precedence associated with each method of specifying the *content state* file
to be used for compilation. This means the path passed through the method with the
highest precedence is used:

```
+-----------------------+-----------------------------------------------+
| Increasing precedence | Method                                        |
+-----------------------+-----------------------------------------------+
|        ||             | Default file `~/.template_content_state.yaml` |
|        ||             +-----------------------------------------------+
|        ||             | `TEMPLATE_CONTENT_STATE_FILE` environment     |
|        ||             | variable                                      |
|        ||             +-----------------------------------------------+
|        \/             | `--content-state` flag                        |
+-----------------------+-----------------------------------------------+
```

#### Default content state file `.template_content_state.yaml`
`txttc` will check if a file named `.template_content_state.yaml` exists in the
current user's home directory. If no other method is used, it will try to
use the contents of this file.

#### `TEMPLATE_CONTENT_STATE_FILE` environment variable

If this environment variable is set when evoking `txttc`, the path it is set to
at that moment will be used as the applicable path to a *content state* file.

#### `--content-state` flag

The path set through this flag will overwrite both previous methods and
will be used as the applicable path to a *content state* file.

## Specifying volatile content

Because the content for *keys* and the selection of *choices* are considered to
usually be volatile, both of them can be specified *after* evoking `txttc`.

To do this a so-called **draft** of the missing content will be opened in the
user's default editor (the editor set in `$EDITOR`. If this environment variable
is missing [`nano`](https://www.nano-editor.org/) is used instead, because anyone
can exit it).
 
Just like a *content state* file, a *draft* uses the [YAML format](https://yaml.org/).

The presentation of the content is similar too; *keys* are presented the same way
as *constants*:

```yaml
keys:
  <name of key>: <content of key>
```

The *choices* for all *options* which appear in a template are made up of
pairs of *option* name and *choice* names: 

```yaml
choices:
  <name of option>: <name of choice for option>
```

Obviously a *choice* name has to be the name of a *choice* which is specified
in the *content state*.

### Filling out a draft

To remove the pain of remembering each *key* and each *choice* that has to be specified
to compile the template, the *draft* will already contain empty entries for all
required elements.

If the template contains defaults for elements, these default also get inserted into
the *draft*, to inform the user about their presence.

Additionally, to not require remembering all *choices* for any *option*, all of them
are inserted into the draft as comments. This means all user is required to do
to select a *choice* is to uncomment it. If multiple *choices* are not commented out
for the name *option*, the last one will be used.

### Persisting a draft

In case one wants to pre-edit, save and then pass a *draft* to `txttc`, the `--draft`
flag is used to only print a template's *draft* and exit.

To pass it back to `txttc`, the `--content` flag accepts a path to an
already filled out *draft* file. No editor will be opened if this flag is used, even
if the given *draft* is missing one or more entries or is otherwise invalid. 

## Miscellaneous flags

### `--ignore-dyn`

If this flag is set dynamic elements will be ignored when compiling the template.
This means that all elements with such special identifiers (e.g. meta constants)
will be treated as regular elements and need to be specified manually.

## Examples

This is the setup for all the following examples:

**`template.txtt`:**

```
key: {testKey},
option: ${testOption},
constant: $testConstant
```

**`content_state.yaml`:**
  
```yaml
constants:
  testConstant: my constant content 
options:
  testOption:
    testChoice: my chosen content
```

### Storing a draft:

```bash
$ txttc --template template.txtt --content-state content_state.yaml --draft > draft.yaml
$ cat draft.yaml
keys:
  # <key>: <content>
  testKey: ''
choices:
  # <option>: <choice>
  # Default literals:
  testOption: ''

  # All available choices (For each option the last choice not commented out will be used):
  # testOption: testChoice    # -> "my chosen content"
```

### Compiling from a filled-out draft:

```bash
# In this case draft.yaml is filled out:
$ cat draft.yaml
keys:
  testKey: my test key content
choices:
  testOption: testChoice
# Now we can compile it:
$ txttc --template template.txtt --content draft.yaml --content-state content_state.yaml > result.txt
# The result will look like this:
$ cat result.txt
key: my test key content,
option: my chosen content,
constant: my constant content
```
