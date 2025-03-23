// Default Rust code for the editor
const DEFAULT_RUST_CODE = `// Define # dimensions
const N: usize = 6;

// Define Optimization Bounds:
let u = SVector::<f64, N>::from_row_slice(&[0.0; N]); // lower bound
let v = SVector::<f64, N>::from_row_slice(&[1.0; N]); // upper bound

// Define the function to mimimize:
fn func<const N: usize>(x: &SVector<f64, N>) -> f64{
    // Example:
    let mut sum = 0.0;
    for i in 0..6{
        sum += (x[i] - 0.12345).powi(2);
    }
    sum
}`;

// Update slider value display
function updateSliderValue(slider) {
    const displayId = slider.getAttribute('data-display-id');
    const specialValueMapping = slider.getAttribute('data-special-value');
    const value = slider.value;

    // Check if this value has a special display value
    if (specialValueMapping && specialValueMapping.includes(':')) {
        const mappings = specialValueMapping.split(',');
        for (const mapping of mappings) {
            const [originalValue, displayValue] = mapping.split(':');
            if (value === originalValue) {
                document.getElementById(displayId).textContent = displayValue;
                return;
            }
        }
    }

    // Default behavior - display the actual value
    document.getElementById(displayId).textContent = value;
}

// Create or update result div
function createResultDiv() {
    let resultDiv = document.getElementById('resultDiv');
    if (!resultDiv) {
        resultDiv = document.createElement('div');
        resultDiv.id = 'resultDiv';
        resultDiv.className = 'container mx-auto p-4 sm:p-6 max-w-4xl mt-4';
        document.querySelector('main').appendChild(resultDiv);
    } else {
        resultDiv.innerHTML = '';
    }
    
    // Style the div
    resultDiv.style.backgroundColor = '#f9f9f9';
    resultDiv.style.borderRadius = '0.5rem';
    resultDiv.style.boxShadow = '0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06)';
    resultDiv.style.padding = '1rem';
    
    return resultDiv;
}

// Display response data in the result div
function displayResponseData(data, resultDiv) {
    // Create elements for each part of the response
    const successEl = document.createElement('div');
    successEl.innerHTML = `<strong>Success:</strong> ${data.success}`;
    resultDiv.appendChild(successEl);
    
    // Add time information
    if (data.time) {
        const timeEl = document.createElement('div');
        const formattedTime = new Date(data.time.secs_since_epoch * 1000).toLocaleString();
        timeEl.innerHTML = `<strong>Time:</strong> ${formattedTime}`;
        resultDiv.appendChild(timeEl);
    }

    if (data.compile_output) {
        const compileEl = document.createElement('div');
        compileEl.innerHTML = `<strong>Compile Output:</strong> ${data.compile_output}`;
        resultDiv.appendChild(compileEl);
    }

    if (data.run_output) {
        const runEl = document.createElement('div');
        runEl.innerHTML = `<strong>Run Output:</strong> <pre>${data.run_output}</pre>`;
        resultDiv.appendChild(runEl);
    }

    if (data.error) {
        const errorEl = document.createElement('div');
        errorEl.innerHTML = `<strong>Error:</strong> <pre>${data.error}</pre>`;
        errorEl.style.color = 'red';
        resultDiv.appendChild(errorEl);
    }
}

// Display error message in the result div
function displayErrorMessage(error, resultDiv) {
    const errorEl = document.createElement('div');
    errorEl.innerHTML = `<strong>Error:</strong> <pre>The backend server does not respond. Likely it is offline.\n${error}</pre>`;
    errorEl.style.color = 'red';
    resultDiv.appendChild(errorEl);
}

// Submit form data to the server
function submitForm(editor) {
    const submitButton = document.getElementById('submit-button-text');
    
    // Prevent multiple submissions
    if (submitButton.textContent === "Loading...") {
        console.log("Please wait for the previous request to complete or refresh the page if something went wrong.");
        return;
    }
    
    // Remove previous results
    let resultDiv = document.getElementById('resultDiv');
    if (resultDiv) {
        resultDiv.remove();
    }
    
    // Update button state
    submitButton.textContent = "Loading...";
    
    // Prepare payload
    const payload = {
        "nsweeps": document.getElementById("nsweeps-slider").value,
        "freach": document.getElementById("freach-slider").value,
        "nf": document.getElementById("nf-slider").value,
        "smax": document.getElementById("smax-slider").value,
        "local": document.getElementById("local-slider").value,
        "code": editor.getValue()
    };

    // Send the code to localhost
    fetch('http://localhost:9090/mcs_form_submit', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(payload)
    })
    .then(response => {
        if (response.ok) {
            return response.json();
        }
        throw new Error('mcs_form_submit: Network response was not ok; likely the backend server is down');
    })
    .then(data => {
        console.log('Response:', data);
        const resultDiv = createResultDiv();
        displayResponseData(data, resultDiv);
        submitButton.textContent = "Evaluate";
    })
    .catch(error => {
        const resultDiv = createResultDiv();
        displayErrorMessage(error, resultDiv);
        submitButton.textContent = "Evaluate";
    });
}

// Initialize Monaco Editor
require.config({ paths: { 'vs': 'https://cdn.jsdelivr.net/npm/monaco-editor@latest/min/vs' } });

require(['vs/editor/editor.main'], function () {
    // Create Monaco editor instance
    const editor = monaco.editor.create(document.getElementById('code-editor'), {
        value: DEFAULT_RUST_CODE,
        language: 'rust',
        theme: 'vs-dark',
        automaticLayout: true,
        minimap: { enabled: false },
        fontSize: 18,
        lineNumbers: 'on',
        roundedSelection: false,
        scrollBeyondLastLine: false,
        padding: { top: 20 },
        margin: 0
    });
    
    // Handle form submission
    document.getElementById('mcs_form').addEventListener('submit', (e) => {
        e.preventDefault();
        submitForm(editor);
    });
});