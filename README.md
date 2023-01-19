# Untitled text templating format and compiler

This repository defines and implements a minimal format for
creating abstract templates of any plain text data.

The original idea was to use these templates to allow
quickly setting up documents with lots of repetitive
text such as emails. It will show if this format is
actually useful for this purpose.

The format offers three types of variable elements: keys, options and constants.
When filling out (compiling) a template, all of these elements will be
translated into a text literal which is inserted in place of the element.
More information on all features provided by the format are found in the
[format specification](spec.md).

## Next up

 - [x] Parse templates
 - [x] Fill out templates
 - [ ] Fill out and manage templates through a CLI tool
  - [ ] Refine the public template API
