// ROGCtl - Main Application JavaScript
// Tauri backend ile iletişim ve UI güncellemeleri

const { invoke } = window.__TAURI__.tauri;
const { appWindow } = window.__TAURI__.window;
const { open } = window.__TAURI__.shell;

// Global state
let updateInterval = null;
let chartData = {
    labels: [],
    cpu: [],
    gpu: []
};

// Initialize app
document.addEventListener('DOMContentLoaded', () => {
    initializeUI();
    initializeEventListeners();
    startAutoUpdate();
    loadSettings();
});

// UI Initialization
function initializeUI() {
    console.log('ROGCtl GUI başlatılıyor...');
    updateStatus();
}

// Event Listeners
function initializeEventListeners() {
    // Header buttons
    document.getElementById('refresh-btn').addEventListener('click', () => updateStatus());
    document.getElementById('settings-btn').addEventListener('click', () => openModal('settings-modal'));
    
    // Window controls
    document.querySelector('.minimize-btn').addEventListener('click', () => {
        appWindow.minimize();
    });
    
    document.querySelector('.close-btn').addEventListener('click', () => {
        const minimizeToTray = localStorage.getItem('minimizeToTray') === 'true';
        if (minimizeToTray) {
            appWindow.hide();
        } else {
            appWindow.close();
        }
    });
    
    // Quick actions
    document.getElementById('view-report').addEventListener('click', async () => {
        await invokeCommand('generate_report');
    });
    
    document.getElementById('nvidia-check').addEventListener('click', async () => {
        await invokeCommand('nvidia_check');
    });
    
    document.getElementById('valorant-fix').addEventListener('click', async () => {
        await invokeCommand('valorant_fix');
    });
    
    document.getElementById('mem-cleanup').addEventListener('click', async () => {
        await invokeCommand('mem_cleanup');
    });
    
    // Footer actions
    document.getElementById('open-logs').addEventListener('click', async () => {
        await invoke('open_logs');
    });
    
    document.getElementById('open-config').addEventListener('click', async () => {
        await invoke('open_config');
    });
    
    // Mode button
    document.getElementById('force-mode-btn').addEventListener('click', () => {
        showModeSelector();
    });
    
    // Modal controls
    document.querySelectorAll('.modal-close, .modal-cancel').forEach(btn => {
        btn.addEventListener('click', () => closeAllModals());
    });
    
    document.querySelector('.modal-save').addEventListener('click', () => {
        saveSettings();
        closeAllModals();
    });
    
    // Settings actions
    document.getElementById('open-config-folder').addEventListener('click', async () => {
        await invoke('open_config_folder');
    });
    
    document.getElementById('restart-daemon').addEventListener('click', async () => {
        if (confirm('Daemon yeniden başlatılsın mı?')) {
            await invoke('restart_daemon');
            setTimeout(() => updateStatus(), 2000);
        }
    });
}

// Auto update
function startAutoUpdate() {
    updateStatus(); // İlk güncelleme
    updateInterval = setInterval(updateStatus, 2000); // Her 2 saniyede bir
}

function stopAutoUpdate() {
    if (updateInterval) {
        clearInterval(updateInterval);
        updateInterval = null;
    }
}

// Update status from backend
async function updateStatus() {
    try {
        const status = await invoke('get_status');
        
        if (status) {
            // Update status values
            document.getElementById('cpu-temp').textContent = status.cpu_temp || '--';
            document.getElementById('cpu-util').textContent = status.cpu_util || '--';
            document.getElementById('gpu-temp').textContent = status.gpu_temp || '--';
            document.getElementById('gpu-util').textContent = status.gpu_util || '--';
            document.getElementById('cpu-fan').textContent = status.cpu_fan || '----';
            document.getElementById('gpu-fan').textContent = status.gpu_fan || '----';
            document.getElementById('gpu-power').textContent = status.gpu_power || '--';
            document.getElementById('power-source').textContent = status.power_source || '--';
            
            // Update mode
            updateModeDisplay(status.mode || 'OFİS', status.mode_details || '');
            
            // Update daemon status
            const statusBadge = document.getElementById('daemon-status');
            if (status.running) {
                statusBadge.className = 'status-badge running';
                statusBadge.innerHTML = '<span class="status-dot"></span>Çalışıyor';
            } else {
                statusBadge.className = 'status-badge stopped';
                statusBadge.innerHTML = '<span class="status-dot"></span>Durduruldu';
            }
            
            // Update footer
            document.getElementById('uptime').textContent = `Çalışma süresi: ${formatUptime(status.uptime_s || 0)}`;
            document.getElementById('last-update').textContent = `Son güncelleme: ${new Date().toLocaleTimeString('tr-TR')}`;
            
            // Update chart
            updateChart(status);
        }
    } catch (error) {
        console.error('Durum güncellenemedi:', error);
        showError('Durum bilgisi alınamadı. Daemon çalışıyor mu?');
    }
}

// Update mode display
function updateModeDisplay(mode, details) {
    const modeElement = document.getElementById('current-mode');
    const detailsElement = document.getElementById('mode-details');
    
    const modeIcons = {
        'BOSTA': '😴',
        'FİLM': '🎬',
        'OFİS': '💼',
        'HAFİF': '🎮',
        'AAA': '🔥',
        'RENDER': '⚙️'
    };
    
    const modeColors = {
        'BOSTA': '#4a9eff',
        'FİLM': '#9d4eff',
        'OFİS': '#00d4ff',
        'HAFİF': '#ffaa00',
        'AAA': '#ff3366',
        'RENDER': '#ff0050'
    };
    
    const icon = modeIcons[mode] || '🎯';
    const color = modeColors[mode] || '#ff0050';
    
    modeElement.querySelector('.mode-icon').textContent = icon;
    modeElement.querySelector('.mode-text').textContent = mode;
    modeElement.style.background = `linear-gradient(135deg, ${color}, ${color}dd)`;
    
    detailsElement.textContent = details;
}

// Update chart
function updateChart(status) {
    const now = new Date().toLocaleTimeString('tr-TR', { hour: '2-digit', minute: '2-digit', second: '2-digit' });
    
    chartData.labels.push(now);
    chartData.cpu.push(parseInt(status.cpu_temp) || 0);
    chartData.gpu.push(parseInt(status.gpu_temp) || 0);
    
    // Son 30 veriyi tut
    if (chartData.labels.length > 30) {
        chartData.labels.shift();
        chartData.cpu.shift();
        chartData.gpu.shift();
    }
    
    drawChart();
}

// Simple chart drawing (canvas API)
function drawChart() {
    const canvas = document.getElementById('temp-chart');
    if (!canvas) return;
    
    const ctx = canvas.getContext('2d');
    const width = canvas.width = canvas.offsetWidth;
    const height = canvas.height = 200;
    
    // Clear
    ctx.clearRect(0, 0, width, height);
    
    if (chartData.cpu.length === 0) return;
    
    // Find max for scaling
    const maxTemp = Math.max(...chartData.cpu, ...chartData.gpu, 100);
    const minTemp = 30;
    const tempRange = maxTemp - minTemp;
    
    // Draw grid
    ctx.strokeStyle = '#333';
    ctx.lineWidth = 1;
    for (let i = 0; i <= 4; i++) {
        const y = (height / 4) * i;
        ctx.beginPath();
        ctx.moveTo(0, y);
        ctx.lineTo(width, y);
        ctx.stroke();
    }
    
    // Draw CPU line
    ctx.strokeStyle = '#ff3366';
    ctx.lineWidth = 2;
    ctx.beginPath();
    chartData.cpu.forEach((temp, i) => {
        const x = (width / (chartData.cpu.length - 1)) * i;
        const y = height - ((temp - minTemp) / tempRange) * height;
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
    });
    ctx.stroke();
    
    // Draw GPU line
    ctx.strokeStyle = '#00d4ff';
    ctx.lineWidth = 2;
    ctx.beginPath();
    chartData.gpu.forEach((temp, i) => {
        const x = (width / (chartData.gpu.length - 1)) * i;
        const y = height - ((temp - minTemp) / tempRange) * height;
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
    });
    ctx.stroke();
    
    // Draw legend
    ctx.font = '12px Segoe UI';
    ctx.fillStyle = '#ff3366';
    ctx.fillText('CPU', 10, 20);
    ctx.fillStyle = '#00d4ff';
    ctx.fillText('GPU', 50, 20);
}

// Invoke command with loading state
async function invokeCommand(command) {
    try {
        const result = await invoke(command);
        showSuccess(result || 'İşlem başarılı');
    } catch (error) {
        showError(`Hata: ${error}`);
    }
}

// Modal functions
function openModal(modalId) {
    document.getElementById(modalId).classList.add('active');
}

function closeAllModals() {
    document.querySelectorAll('.modal').forEach(modal => {
        modal.classList.remove('active');
    });
}

// Mode selector
function showModeSelector() {
    const modes = [
        { key: 'bosta', name: 'BOŞTA', icon: '😴' },
        { key: 'film', name: 'FİLM', icon: '🎬' },
        { key: 'ofis', name: 'OFİS', icon: '💼' },
        { key: 'hafif', name: 'HAFİF OYUN', icon: '🎮' },
        { key: 'aaa', name: 'AAA OYUN', icon: '🔥' },
        { key: 'render', name: 'RENDER', icon: '⚙️' }
    ];
    
    // TODO: Create a proper mode selector modal
    const modeText = modes.map((m, i) => `${i + 1}. ${m.icon} ${m.name}`).join('\n');
    const choice = prompt(`Mod seçin:\n\n${modeText}\n\n(Otomatik için 0)`);
    
    if (choice !== null) {
        const index = parseInt(choice) - 1;
        if (index === -1) {
            invoke('set_mode', { mode: 'auto' });
        } else if (index >= 0 && index < modes.length) {
            invoke('set_mode', { mode: modes[index].key });
        }
    }
}

// Settings
function loadSettings() {
    document.getElementById('autostart-check').checked = 
        localStorage.getItem('autostart') === 'true';
    document.getElementById('minimize-tray-check').checked = 
        localStorage.getItem('minimizeToTray') === 'true';
    document.getElementById('notifications-check').checked = 
        localStorage.getItem('notifications') !== 'false';
}

function saveSettings() {
    localStorage.setItem('autostart', 
        document.getElementById('autostart-check').checked);
    localStorage.setItem('minimizeToTray', 
        document.getElementById('minimize-tray-check').checked);
    localStorage.setItem('notifications', 
        document.getElementById('notifications-check').checked);
    
    showSuccess('Ayarlar kaydedildi');
}

// Utilities
function formatUptime(seconds) {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    return `${hours}s ${minutes}d`;
}

function showSuccess(message) {
    // TODO: Implement toast notifications
    console.log('✓', message);
}

function showError(message) {
    // TODO: Implement toast notifications
    console.error('✗', message);
}

// Cleanup on unload
window.addEventListener('beforeunload', () => {
    stopAutoUpdate();
});
