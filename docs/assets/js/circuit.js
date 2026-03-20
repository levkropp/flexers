// Animated circuit board background for Flexers
// ESP32-inspired: blue and gold traces flowing through the background

const canvas = document.getElementById('circuit-bg');
const ctx = canvas.getContext('2d');

let width = canvas.width = window.innerWidth;
let height = canvas.height = window.innerHeight;

// Resize handler
window.addEventListener('resize', () => {
    width = canvas.width = window.innerWidth;
    height = canvas.height = window.innerHeight;
});

// Circuit node
class Node {
    constructor(x, y) {
        this.x = x;
        this.y = y;
        this.connections = [];
        this.pulse = 0;
        this.pulseSpeed = 0.02 + Math.random() * 0.03;
    }

    update() {
        this.pulse += this.pulseSpeed;
        if (this.pulse > Math.PI * 2) this.pulse = 0;
    }

    draw() {
        // Draw node
        const brightness = Math.sin(this.pulse) * 0.5 + 0.5;
        const size = 2 + brightness * 2;

        ctx.beginPath();
        ctx.arc(this.x, this.y, size, 0, Math.PI * 2);
        ctx.fillStyle = `rgba(33, 150, 243, ${0.3 + brightness * 0.4})`;
        ctx.fill();

        // Glow
        ctx.beginPath();
        ctx.arc(this.x, this.y, size * 2, 0, Math.PI * 2);
        ctx.fillStyle = `rgba(33, 150, 243, ${brightness * 0.1})`;
        ctx.fill();
    }
}

// Circuit trace
class Trace {
    constructor(from, to) {
        this.from = from;
        this.to = to;
        this.progress = Math.random();
        this.speed = 0.005 + Math.random() * 0.01;
        this.isGold = Math.random() < 0.3; // 30% gold traces
    }

    update() {
        this.progress += this.speed;
        if (this.progress > 1) this.progress = 0;
    }

    draw() {
        // Draw static trace
        ctx.beginPath();
        ctx.moveTo(this.from.x, this.from.y);
        ctx.lineTo(this.to.x, this.to.y);
        ctx.strokeStyle = this.isGold
            ? 'rgba(255, 193, 7, 0.1)'
            : 'rgba(33, 150, 243, 0.1)';
        ctx.lineWidth = 1;
        ctx.stroke();

        // Draw moving pulse
        const x = this.from.x + (this.to.x - this.from.x) * this.progress;
        const y = this.from.y + (this.to.y - this.from.y) * this.progress;

        const gradient = ctx.createRadialGradient(x, y, 0, x, y, 15);
        if (this.isGold) {
            gradient.addColorStop(0, 'rgba(255, 193, 7, 0.6)');
            gradient.addColorStop(1, 'rgba(255, 193, 7, 0)');
        } else {
            gradient.addColorStop(0, 'rgba(33, 150, 243, 0.6)');
            gradient.addColorStop(1, 'rgba(33, 150, 243, 0)');
        }

        ctx.beginPath();
        ctx.arc(x, y, 15, 0, Math.PI * 2);
        ctx.fillStyle = gradient;
        ctx.fill();
    }
}

// Generate circuit grid
const nodes = [];
const traces = [];
const gridSize = 150;
const nodeCount = Math.ceil(width / gridSize) * Math.ceil(height / gridSize);

// Create nodes in a grid with some randomness
for (let x = gridSize / 2; x < width; x += gridSize) {
    for (let y = gridSize / 2; y < height; y += gridSize) {
        const offsetX = (Math.random() - 0.5) * 60;
        const offsetY = (Math.random() - 0.5) * 60;
        nodes.push(new Node(x + offsetX, y + offsetY));
    }
}

// Connect nearby nodes
nodes.forEach((node, i) => {
    nodes.forEach((other, j) => {
        if (i !== j) {
            const dx = node.x - other.x;
            const dy = node.y - other.y;
            const dist = Math.sqrt(dx * dx + dy * dy);

            // Connect if close enough and not already connected
            if (dist < gridSize * 1.5 && Math.random() < 0.4) {
                if (!node.connections.includes(j)) {
                    node.connections.push(j);
                    traces.push(new Trace(node, other));
                }
            }
        }
    });
});

// Animation loop
function animate() {
    ctx.fillStyle = 'rgba(10, 10, 15, 0.3)';
    ctx.fillRect(0, 0, width, height);

    // Update and draw traces
    traces.forEach(trace => {
        trace.update();
        trace.draw();
    });

    // Update and draw nodes
    nodes.forEach(node => {
        node.update();
        node.draw();
    });

    requestAnimationFrame(animate);
}

// Start animation
animate();

// Add some random "data bursts" for visual interest
setInterval(() => {
    if (Math.random() < 0.3) {
        const randomNode = nodes[Math.floor(Math.random() * nodes.length)];
        randomNode.pulse = 0;
        randomNode.pulseSpeed = 0.05 + Math.random() * 0.05;
    }
}, 500);
