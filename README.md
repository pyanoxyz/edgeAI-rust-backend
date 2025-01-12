# EdgeAI Rust Backend

An advanced Rust-based backend service for handling AI/ML operations with features like RAG (Retrieval Augmented Generation), pair programming assistance, code analysis, and local LLM integration
for edge devices.
This Backend also suports our own AI Copilot vs code extension.

## 🌟 Features

- **Local LLM Integration**: Connect and manage local LLM instances for AI operations
- **RAG (Retrieval Augmented Generation)**: Context-aware responses using document embeddings
- **Pair Programming**: AI-assisted code generation and review
- **Code Analysis**: Parse and analyze code across multiple languages
- **Real-time Streaming**: Stream AI responses for better user experience
- **Embeddings & Reranking**: Generate embeddings and rerank search results
- **SQLite Integration**: Efficient storage and retrieval of embeddings and chat history

## 🔧 Architecture

The project is organized into several key modules:

```
src/
├── database/          # Database operations and configurations for embedded sqlite.
├── embeddings/        # Text embedding generation
├── context/          # Context management for RAG
├── infill/           # Code infilling operations - Seperate 1.5-3b LLM for auto code completion.
├── model_state/      # LLM model state management - Models being decided on the Edge condifuration.
├── pair_programmer/  # Pair programming functionality - Agents colloborating to solve Complex code problems
├── parser/           # Code parsing and analysis
├── rag/             # RAG implementation
└── chats/           # Chat operations and history
```

## 🚀 Getting Started

### Prerequisites

- Rust 1.70+ 
- SQLite3
- Python 3.8+ (for model dependencies)
- Tree-sitter (for code parsing)

### Installation

1. Clone the repository:
```bash
git clone https://github.com/yourusername/edgeAI-rust-backend.git
cd edgeAI-rust-backend
```

2. Create necessary directories:
```bash
mkdir -p ~/.pyano/{models,configs,scripts,parsers,database,indexes}
```

3. Install dependencies:
```bash
cargo build
```

4. Set up environment variables in `.env`:
```env
RUST_LOG=info
LOCAL_URL=http://localhost:52555
INFILL_LOCAL_URL=http://localhost:52554
TEMPERATURE=0.7
TOP_K=20
TOP_P=0.8
```

### Running the Server

```bash
cargo run
```

The server will start at `localhost:52556` by default.

## 📚 API Endpoints

### Chat Operations
- `POST /chat`: General chat endpoint
- `POST /chat/explain`: Code explanation
- `POST /chat/refactor`: Code refactoring suggestions
- `POST /chat/find-bugs`: Bug detection
- `POST /chat/tests-cases`: Test case generation
- `POST /chat/docstring`: Documentation generation

### RAG Operations
- `POST /rags/index/code`: Index code for RAG
- `GET /rags/index/code`: Get indexed context
- `POST /rags/index/fetch-context`: Fetch similar code contexts
- `DELETE /rags/index/code`: Remove indexed context

### Pair Programming
- `POST /pair-programmer/generate-steps`: Generate coding steps
- `GET /pair-programmer/steps/{pair_programmer_id}`: Get generated steps
- `POST /pair-programmer/steps/execute`: Execute a coding step
- `POST /pair-programmer/steps/chat`: Chat about a specific step

## 🔍 Key Components

### Database
- Uses SQLite for storing embeddings, chat history, and context
- Implements efficient vector search capabilities
- Handles session management and chat history

### Embedding Generation
- Generates embeddings for code snippets and text
- Uses FastEmbed for efficient embedding generation
- Supports reranking of search results

### Code Parsing
- Uses Tree-sitter for robust code parsing
- Supports multiple programming languages
- Generates structured code representations

### LLM Integration
- Manages local LLM instances
- Handles streaming responses
- Supports both local and cloud execution modes

## 🤝 Contributing

We welcome contributions! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## 📝 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [Rust-Bert](https://github.com/guillaume-be/rust-bert) for text embedding generation
- [Tree-sitter](https://tree-sitter.github.io/tree-sitter/) for code parsing
- [SQLite](https://www.sqlite.org/) for database management
- [FastEmbed](https://github.com/Anush008/fastembed) for efficient embedding operations

## 📬 Contact

For questions and feedback, please open an issue in the GitHub repository.