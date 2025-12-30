// Hello Plugin - Rust Example
// Simple JavaScript bundle for the plugin frontend

(function() {
    console.log('[HelloPlugin] Loading frontend bundle...');

    const pluginId = 'hello-plugin-rust';

    // Mount function - called when navigating to plugin view
    window.ToruPlugins = window.ToruPlugins || {};
    window.ToruPlugins[pluginId] = {
        mount(container, api) {
            console.log('[HelloPlugin] Mounting to container:', container);

            // Clear container
            container.innerHTML = '';

            // Create header
            const header = document.createElement('div');
            header.className = 'text-center mb-8';
            header.innerHTML = `
                <h1 class="text-3xl font-bold mb-2">🦀 Hello World (Rust)</h1>
                <p class="text-gray-600">A simple Toru plugin written in Rust</p>
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
                        <span class="font-medium">Rust</span>
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

            // Create message counter
            const counter = document.createElement('div');
            counter.className = 'bg-white rounded-lg shadow p-6 mb-6';
            counter.innerHTML = `
                <h2 class="text-xl font-semibold mb-4">Message Counter</h2>
                <div class="flex items-center space-x-4">
                    <button id="increment-btn" class="bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700 transition">
                        Increment
                    </button>
                    <div id="counter-value" class="text-2xl font-bold">0</div>
                </div>
                <p class="text-sm text-gray-500 mt-2">Click the button to increment the counter (stored in plugin KV)</p>
            `;
            container.appendChild(counter);

            // Add event listener for increment button
            document.getElementById('increment-btn').addEventListener('click', async () => {
                const btn = document.getElementById('increment-btn');
                btn.disabled = true;
                btn.textContent = 'Incrementing...';

                try {
                    // Simulate increment (in real plugin, this would call plugin API)
                    let currentValue = parseInt(document.getElementById('counter-value').textContent);
                    const newValue = currentValue + 1;
                    document.getElementById('counter-value').textContent = newValue;

                    api.showToast('Counter incremented!', 'success');
                } catch (error) {
                    console.error('[HelloPlugin] Error incrementing:', error);
                    api.showToast('Failed to increment counter', 'error');
                } finally {
                    btn.disabled = false;
                    btn.textContent = 'Increment';
                }
            });

            // Create test API section
            const apiSection = document.createElement('div');
            apiSection.className = 'bg-white rounded-lg shadow p-6';
            apiSection.innerHTML = `
                <h2 class="text-xl font-semibold mb-4">Test Plugin API</h2>
                <div class="space-y-4">
                    <button id="test-api-btn" class="bg-green-600 text-white px-4 py-2 rounded hover:bg-green-700 transition">
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
                    const response = await api.fetch('/api/plugins/hello-plugin-rust', {
                        method: 'GET',
                    });

                    const data = await response.json();
                    responseDiv.textContent = JSON.stringify(data, null, 2);
                    api.showToast('API call successful!', 'success');
                } catch (error) {
                    console.error('[HelloPlugin] Error calling API:', error);
                    responseDiv.textContent = 'Error: ' + error.message;
                    api.showToast('Failed to call API', 'error');
                } finally {
                    btn.disabled = false;
                    btn.textContent = 'Call Plugin API';
                }
            });

            console.log('[HelloPlugin] Mounted successfully');
        },

        unmount(container) {
            console.log('[HelloPlugin] Unmounting from container:', container);
            container.innerHTML = '';
        }
    };

    console.log('[HelloPlugin] Frontend bundle loaded');
})();
