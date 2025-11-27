# Skabelon

Simple templating engine for rust. 

Uses angular syntax, so you will be able to use the build in angular parser from prettier for formatting.

**Note that template logic is not supported**. Calculate logic in your rust files.

## Features
- Angular syntax
- Templates are parsed at runtime. Templates can be recalculated with `templates.reload()`.
- `@if() {} @else if() {} @else {}` support.
- `@for() {}` support for iteration.
- `@include {}` support for partials. Partials can have a `{{ content }}` where block from `@include` will be rendered.
- `object["value"]` or `object.value` for accessing object values

## Usage
Templates can be loaded with a glob or string.

```rust
let mut templates = Templates::new();
templates.load_glob("templates/**/*.html");

let template_str =
    "<table>@for(row in table) {<tr>@for(col in row) {<td>{{col}}</td>}</tr>}</table>";
templates.load_str("template", template_str);
```

### Context notation
Context can be referenced in templates with `{{ key }}`.
If key is an object, values can be referenced with `{{ key["value"] }}` or `{{ key.value }}`

### `@if`

#### Renders block if condition is true.

```html
@if (condition) {
  block
}
```

#### If else
```html
@if (condition) {
  block
} @else {
  otherwhere
}
```

#### if else if 
```html
@if (condition) {
  block
} @else if (condition) {
  block
}
```

#### if else if else
```html
@if (condition) {
  block
} @else if (condition) {
  block
} @else {
  otherwhere
}
```

### `@for`
Iterates over array.

```html
@for (item in items) {
  {{item}}
}
```

### `@include`
#### Includes other template by key.

```html
@include (key) {}
```

#### Render block in partial content slot

`main`
```html
@include (partial) {Hello}
```

`partial`
```html
{{ content }} World
```

#### Context can be send to partial.

```html
@include (key; value="hello") {}
```

or for variables
```html
@include (key; value=variable) {}
```

