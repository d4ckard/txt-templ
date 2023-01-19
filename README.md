# Untitled text templating format and compiler.

This repository defines and implements a minimal format for
creating abstract templates of any plain text data.

The original idea was to use these templates to allow
quickly setting up documents with lots of repetitive
text such as emails. It will show however, if this format is
actually useful for this puropose.

The format offers three types of variable elements: keys, options and constants.
When filling out (compiling) a template all of these elements will be
translated into a text literal which is inserted in place of the element.
More information on all options provided by the format are found in the
[format specification](spec.md).

## Next up

 - [x] Parse templates and fill them out
 - [ ] Fill out and manage templates through a cli tool
