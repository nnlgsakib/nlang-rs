const API_BASE = 'http://127.0.0.1:8082';

function logCombat(message) {
    const log = document.getElementById('combat-log');
    const p = document.createElement('p');
    p.textContent = `[${new Date().toLocaleTimeString()}] ${message}`;
    log.appendChild(p);
    log.scrollTop = log.scrollHeight;
}

async function loadCharacter() {
    try {
        logCombat('Fetching character data...');
        const response = await fetch(`${API_BASE}/api/character`);
        const data = await response.json();
        
        document.getElementById('player-name').textContent = data.name;
        document.getElementById('player-level').textContent = data.level;
        document.getElementById('player-health').textContent = data.health;
        document.getElementById('player-max-health').textContent = data.max_health;
        
        const healthPercent = (data.health / data.max_health) * 100;
        document.getElementById('health-bar').style.width = `${healthPercent}%`;
        
        logCombat(`Character loaded: ${data.name} (Level ${data.level})`);
    } catch (error) {
        logCombat(`ERROR: Failed to load character - ${error.message}`);
    }
}

async function loadStats() {
    try {
        logCombat('Fetching stats data...');
        const response = await fetch(`${API_BASE}/api/stats`);
        const data = await response.json();
        
        document.getElementById('stat-attack').textContent = data.attack;
        document.getElementById('stat-defense').textContent = data.defense;
        document.getElementById('stat-luck').textContent = data.luck;
        document.getElementById('stat-crit').textContent = `${data.crit_chance}%`;
        
        logCombat(`Stats updated: ATK=${data.attack}, DEF=${data.defense}, LUCK=${data.luck}`);
    } catch (error) {
        logCombat(`ERROR: Failed to load stats - ${error.message}`);
    }
}

async function simulateCombat() {
    try {
        logCombat('>>> Starting combat simulation...');
        const response = await fetch(`${API_BASE}/api/combat`);
        const data = await response.json();
        
        logCombat(`⚔️ Combat Round ${data.round}`);
        logCombat(`   Hit Chance: ${data.hit_chance}%`);
        logCombat(`   Damage Dealt: ${data.damage}`);
        logCombat(`   Critical Hit: ${data.is_critical ? 'YES' : 'NO'}`);
        logCombat(`   Result: ${data.result}`);
        logCombat('>>> Combat simulation complete!');
    } catch (error) {
        logCombat(`ERROR: Failed to simulate combat - ${error.message}`);
    }
}

async function performEmote(emote) {
    try {
        logCombat(`Performing emote: ${emote}...`);
        const response = await fetch(`${API_BASE}/api/emote/${emote}`);
        const data = await response.json();
        
        logCombat(`${data.message}`);
    } catch (error) {
        logCombat(`ERROR: Failed to perform emote - ${error.message}`);
    }
}

async function checkServerStatus() {
    try {
        const response = await fetch(`${API_BASE}/api/status`);
        const data = await response.json();
        document.getElementById('server-status').className = 'status-online';
        logCombat(`Server online: ${data.status} (version ${data.version})`);
    } catch (error) {
        document.getElementById('server-status').className = 'status-offline';
        logCombat('Server offline or unreachable');
    }
}

window.onload = function() {
    logCombat('=== RPG Game Web Edition Initialized ===');
    logCombat('Welcome to the web-based RPG game!');
    checkServerStatus();
    loadCharacter();
    loadStats();
};
