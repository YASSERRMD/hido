import gradio as gr
import hido
import json

# Global state
did_manager = hido.DIDManager()
# Initialize blockchain backend (mocked or real depending on config)
try:
    audit_backend = hido.AuditBackend.create_blockchain()
    backend_status = "✅ Audit Backend Connected (Blockchain)"
except Exception as e:
    audit_backend = None
    backend_status = f"⚠️ Audit Backend Error: {e}"

def generate_identity():
    did = did_manager.generate()
    return did, f"Identity generated successfully"

def resolve_identity(did):
    try:
        doc = did_manager.resolve(did)
        doc_json = json.loads(doc.to_json())
        return json.dumps(doc_json, indent=2)
    except Exception as e:
        return f"Error resolving DID: {str(e)}"

def create_intent(action, domain, target, priority):
    try:
        priority_map = {"Low": 0, "Normal": 1, "High": 2, "Critical": 3}
        p_val = priority_map.get(priority, 1)
        
        intent = hido.Intent(action, domain)
        intent.set_target(target)
        intent.set_priority(p_val)
        
        intent_json = json.loads(intent.to_json())
        return json.dumps(intent_json, indent=2)
    except Exception as e:
        return {"error": str(e)}

def record_audit(actor, action, target):
    if not audit_backend:
        return "Audit backend not available."
        
    try:
        entry_id = audit_backend.record(actor, action, target)
        return f"✅ Audit Recorded!\nEntry ID: {entry_id}\nBackend: {audit_backend.backend_type()}"
    except Exception as e:
        return f"Error recording audit: {str(e)}"

# UI Layout
theme = gr.themes.Soft(
    primary_hue="purple",
    secondary_hue="indigo",
)

with gr.Blocks(theme=theme, title="HIDO Dashboard") as demo:
    gr.Markdown(
        """
        # 🛡️ HIDO Agent Dashboard
        **Hierarchical Intent-Driven Orchestration Control Plane**
        
        This dashboard demonstrates the Python bindings for the HIDO Core Rust library.
        """
    )
    
    with gr.Tab("🔐 Identity (UAIL)"):
        gr.Markdown("### Universal Agent Identity Layer")
        with gr.Row():
            with gr.Column():
                gen_btn = gr.Button("Generate New Agent Identity", variant="primary")
                status_output = gr.Label(label="Status")
            with gr.Column():
                did_output = gr.Textbox(label="Generated DID", show_copy_button=True)
        
        gen_btn.click(generate_identity, outputs=[did_output, status_output])
        
        gr.Markdown("---")
        gr.Markdown("### DID Resolver")
        with gr.Row():
            resolve_input = gr.Textbox(label="DID to Resolve", placeholder="did:hido:...")
            resolve_btn = gr.Button("Resolve Document")
        
        doc_output = gr.Code(language="json", label="DID Document")
        resolve_btn.click(resolve_identity, inputs=[resolve_input], outputs=[doc_output])

    with gr.Tab("🧠 Intents (ICC)"):
        gr.Markdown("### Intent Communication Channel")
        with gr.Row():
            action_in = gr.Textbox(label="Action", value="analyze_data")
            domain_in = gr.Textbox(label="Domain", value="finance")
        
        with gr.Row():
            target_in = gr.Textbox(label="Target Resource", value="s3://lake/financial_records.parquet")
            priority_in = gr.Dropdown(["Low", "Normal", "High", "Critical"], label="Priority", value="Normal")
            
        create_btn = gr.Button("Construct Semantic Intent", variant="primary")
        intent_output = gr.JSON(label="Constructed Intent Object")
        
        create_btn.click(create_intent, inputs=[action_in, domain_in, target_in, priority_in], outputs=[intent_output])

    with gr.Tab("📜 Audit (BAL)"):
        gr.Markdown("### Blockchain Audit Layer")
        gr.Markdown(f"*{backend_status}*")
        
        with gr.Row():
            audit_actor = gr.Textbox(label="Actor DID (Copy from Identity tab)")
            audit_action = gr.Textbox(label="Action Performed", value="data_access")
            audit_target = gr.Textbox(label="Target Resource", value="sensitive_db_table")
            
        record_btn = gr.Button("Record Immutable Log", variant="stop")
        audit_output = gr.Textbox(label="Blockchain Receipt")
        
        record_btn.click(record_audit, inputs=[audit_actor, audit_action, audit_target], outputs=[audit_output])

if __name__ == "__main__":
    print("Starting HIDO Dashboard...")
    print("Ensure you have installed the package: maturin develop")
    print("Ensure you have installed gradio: pip install gradio")
    demo.launch()
