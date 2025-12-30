// Hello Plugin - Python Example
// Simple JavaScript bundle for the plugin frontend

(function() {
    console.log('[HelloPlugin-Python] Loading frontend bundle...');

    const pluginId = 'hello-plugin-python';

    // Mount function - called when navigating to plugin view
    window.ToruPlugins = window.ToruPlugins || {};
    window.ToruPlugins[pluginId] = {
        mount(container, api) {
            console.log('[HelloPlugin-Python] Mounting to container:', container);

            // Clear container
            container.innerHTML = '';

            // Create header
            const header = document.createElement('div');
            header.className = 'text-center mb-8';
            header.innerHTML = `
                <h1 class="text-3xl font-bold mb-2">🐍 Hello World (Python)</h1>
                <p class="text-gray-600">A simple Toru plugin written in Python</p>
            `;
            container.appendChild(header);

            // Create status card
            const statusCard = document.createElement('div');
            statusCard.className = 'bg-white rounded-lg shadow p-6 mb-6';
            statusCard.innerHTML = `
                <h2 class="text-xl font-semibold mb-4">Plugin Status</h2>
                <div class="space-y-2">
                    <div class="flex justify-between">
                        <span class="text-gray-600">Language:</span>
                        <span class="font-medium">Python</span>
                    </div>
                    <div class="flex justify-between">
                        <span class="text-gray-600">Version:</span>
                        <span class="font-medium">0.1.0</span>
                    </div>
                    <div class="flex justify-between">
                        <span class="text-gray-600">Status:</span>
                        <span class="text-green-600 font-medium">Running</span>
                    </div>
                </div>
            `;
            container.appendChild(statusCard);

            // Create features section
            const featuresCard = document.createElement('div');
            featuresCard.className = 'bg-white rounded-lg shadow p-6 mb-6';
            featuresCard.innerHTML = `
                <h2 class="text-xl font-semibold mb-4">Python Plugin Features</h2>
                <ul class="space-y-2 text-gray-700">
                    <li class="flex items-center">
                        <span class="text-green-500 mr-2">✓</span>
                        Unix socket communication
                    </li>
                    <li class="flex items-center">
                        <span class="text-green-500 mr-2">✓</span>
                        JSON message protocol
                    </li>
                    <li class="flex items-center">
                        <span class="text-green-500 mr-2">✓</span>
                        HTTP request handling
                    </li>
                    <li class="flex items-center">
                        <span class="text-green-500 mr-2">✓</span>
                        Key-value storage
                    </li>
                    <li class="flex items-center">
                        <span class="text-green-500 mr-2">✓</span>
                        Frontend bundle serving
                    </li>
                </ul>
            `;
            container.appendChild(featuresCard);

            // Create test API section
            const apiSection = document.createElement('div');
            apiSection.className = 'bg-white rounded-lg shadow p-6';
            apiSection.innerHTML = `
                <h2 class="text-xl font-semibold mb-4">Test Plugin API</h2>
                <div class="space-y-4">
                    <button id="test-api-btn" class="bg-purple-600 text-white px-4 py-2 rounded hover:bg-purple-700 transition">
                        Call Plugin API
                    </button>
                    <div id="api-response" class="mt-4 p-4 bg-gray-100 rounded font-mono text-sm hidden">
                        Response will appear here...
                    </div>
                </div>
            `;
            container.appendChild(apiSection);

            // Add event listener for test API button
            document.getElementById('test-api-btn').addEventListener('click', async () => {
                const btn = document.getElementById('test-api-btn');
                const responseDiv = document.getElementById('api-response');
                btn.disabled = true;
                btn.textContent = 'Calling...';
                responseDiv.classList.remove('hidden');
                responseDiv.textContent = 'Loading...';

                try {
                    const response = await api.fetch('/api/plugins/hello-plugin-python', {
                        method: 'GET',
                    });

                    const data = await response.json();
                    responseDiv.textContent = JSON.stringify(data, null, 2);
                    api.showToast('API call successful!', 'success');
                } catch (error) {
                    console.error('[HelloPlugin-Python] Error calling API:', error);
                    responseDiv.textContent = 'Error: ' + error.message;
                    api.showToast('Failed to call API', 'error');
                } finally {
                    btn.disabled = false;
                    btn.textContent = 'Call Plugin API';
                }
            });

            console.log('[HelloPlugin-Python] Mounted successfully');
        },

        unmount(container) {
            console.log('[HelloPlugin-Python] Unmounting from container:', container);
            container.innerHTML = '';
        }
    };

    console.log('[HelloPlugin-Python] Frontend bundle loaded');
})();
