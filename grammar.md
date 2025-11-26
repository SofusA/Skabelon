# Context notation
Context can be referenced in templates with `{{ key }}`.
If key is an object, values can be referenced with `{{ key["value"] }}` or `{{ key.value }}`

# `@if`

Renders block if condition is true.

```html
@if (condition) {
  block
}
```

If else
```html
@if (condition) {
  block
} @else {
  otherwhere
}
```
if else if 
```html
@if (condition) {
  block
} @else if (condition) {
  block
}
```

if else if else
```html
@if (condition) {
  block
} @else if (condition) {
  block
} @else {
  otherwhere
}
```

# `@for`
Iterates over array.

```html
@for (item in items) {
  {{item}}
}
```

# `@include`
Includes other template by key.

```html
@include (key) {}
```

Render block in partial content slot

`main`
```html
@include (partial) {Hello}
```

`partial`
```html
<content-slot> World
```

Context can be send to partial.

```html
@include (key; value="hello") {}
```

