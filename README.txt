# Registry


# App File Format

# Component File Format

- constants are not parsed for JSONPath, data which may conflict with JSONPath can be put here and referenced. 
- $$ means the output of the referenced component. Outputs are also not parsed for JSONPath.
- Any component whose output is not used as an input is assumed to be outputting something to be displayed.