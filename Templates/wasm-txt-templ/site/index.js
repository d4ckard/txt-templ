import("./node_modules/wasm-txt-templ/wasm_txt_templ.js").then(module => {
  try {
    // Load a template by parsing it.
    const input = "Hallo {name}";
    document.getElementById("input").innerHTML = input;
    const template = module.Template.parse(input);

    // Use the user's content to fill out the missing values
    // in the loaded template.
    // const output = template.fill_out(/* user_content, user_content_state*/);
    // document.getElementById("output").innerHTML = output;
  } catch(e) {
    console.error(e);
  }
});
