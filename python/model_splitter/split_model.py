#!/usr/bin/env python3
"""
Model Splitter for ChatLoop

This script splits a large LLM into layer-group partitions for distributed inference.
It converts models to Safetensors format and generates partition metadata.

Usage:
    python split_model.py --model meta-llama/Llama-2-7b-hf --output ./partitions --num-partitions 4
"""

import argparse
import json
import shutil
from pathlib import Path
from typing import Dict, List, Tuple

import torch
from safetensors.torch import save_file
from transformers import AutoModelForCausalLM, AutoTokenizer


def parse_args():
    parser = argparse.ArgumentParser(description="Split LLM for distributed inference")
    parser.add_argument(
        "--model",
        type=str,
        required=True,
        help="Model name or path (HuggingFace model ID or local path)",
    )
    parser.add_argument(
        "--output",
        type=str,
        required=True,
        help="Output directory for partitions",
    )
    parser.add_argument(
        "--num-partitions",
        type=int,
        default=4,
        help="Number of layer group partitions",
    )
    parser.add_argument(
        "--quantization",
        type=str,
        choices=["none", "int8", "int4"],
        default="none",
        help="Quantization type",
    )
    parser.add_argument(
        "--tokenizer-output",
        type=str,
        default=None,
        help="Where to save tokenizer (defaults to output directory)",
    )
    return parser.parse_args()


def get_layer_groups(model, num_partitions: int) -> List[Tuple[int, int]]:
    """
    Calculate layer group boundaries.

    Args:
        model: The model to split
        num_partitions: Number of partitions to create

    Returns:
        List of (start_layer, end_layer) tuples
    """
    # Try to determine number of layers
    if hasattr(model, "model") and hasattr(model.model, "layers"):
        num_layers = len(model.model.layers)
    elif hasattr(model, "transformer") and hasattr(model.transformer, "h"):
        num_layers = len(model.transformer.h)
    elif hasattr(model, "gpt_neox") and hasattr(model.gpt_neox, "layers"):
        num_layers = len(model.gpt_neox.layers)
    else:
        raise ValueError("Cannot determine model architecture")

    print(f"Model has {num_layers} layers")

    # Calculate layer groups
    layers_per_partition = (num_layers + num_partitions - 1) // num_partitions

    layer_groups = []
    for i in range(num_partitions):
        start = i * layers_per_partition
        end = min((i + 1) * layers_per_partition, num_layers)
        if start < num_layers:
            layer_groups.append((start, end))

    print(f"Created {len(layer_groups)} layer groups:")
    for i, (start, end) in enumerate(layer_groups):
        print(f"  Partition {i}: layers {start}-{end}")

    return layer_groups


def get_layer_prefix(layer_idx: int, model) -> str:
    """Get the prefix for layer weights based on model architecture."""
    if hasattr(model, "model") and hasattr(model.model, "layers"):
        return f"model.layers.{layer_idx}"
    elif hasattr(model, "transformer") and hasattr(model.transformer, "h"):
        return f"transformer.h.{layer_idx}"
    elif hasattr(model, "gpt_neox") and hasattr(model.gpt_neox, "layers"):
        return f"gpt_neox.layers.{layer_idx}"
    else:
        raise ValueError("Unknown model architecture")


def get_embeddings(model) -> Dict[str, torch.Tensor]:
    """Extract embedding layers."""
    embeddings = {}

    if hasattr(model, "model") and hasattr(model.model, "embed_tokens"):
        embeddings["model.embed_tokens.weight"] = model.model.embed_tokens.weight.data
    elif hasattr(model, "transformer") and hasattr(model.transformer, "wte"):
        embeddings["transformer.wte.weight"] = model.transformer.wte.weight.data
    elif hasattr(model, "gpt_neox") and hasattr(model.gpt_neox, "embed_in"):
        embeddings["gpt_neox.embed_in.weight"] = model.gpt_neox.embed_in.weight.data

    return embeddings


def get_lm_head(model) -> Dict[str, torch.Tensor]:
    """Extract language model head."""
    lm_head = {}

    if hasattr(model, "lm_head"):
        lm_head["lm_head.weight"] = model.lm_head.weight.data

    return lm_head


def get_norm_layers(model) -> Dict[str, torch.Tensor]:
    """Extract normalization layers."""
    norms = {}

    if hasattr(model, "model") and hasattr(model.model, "norm"):
        norms["model.norm.weight"] = model.model.norm.weight.data

    return norms


def split_partition(
    state_dict: Dict[str, torch.Tensor],
    layer_group: Tuple[int, int],
    partition_idx: int,
    model,
) -> Dict[str, torch.Tensor]:
    """
    Extract a single layer partition from the full state dict.

    Args:
        state_dict: Full model state dict
        layer_group: (start_layer, end_layer) tuple
        partition_idx: Index of this partition
        model: The model (for architecture detection)

    Returns:
        State dict containing only this partition's weights
    """
    partition_state = {}
    start_layer, end_layer = layer_group

    # Extract layer weights
    for layer_idx in range(start_layer, end_layer):
        prefix = get_layer_prefix(layer_idx, model)

        for key, value in state_dict.items():
            if key.startswith(prefix):
                partition_state[key] = value

    # First partition gets embeddings
    if partition_idx == 0:
        partition_state.update(get_embeddings(model))

    # Last partition gets LM head and final norm
    if partition_idx == len(get_layer_groups(model, (end_layer - start_layer))) - 1:
        partition_state.update(get_lm_head(model))
        partition_state.update(get_norm_layers(model))

    return partition_state


def quantize_int8(tensor: torch.Tensor) -> Tuple[torch.Tensor, float]:
    """
    Quantize a tensor to INT8.

    Args:
        tensor: Input tensor

    Returns:
        (quantized_tensor, scale)
    """
    # Find min and max
    min_val = tensor.min().item()
    max_val = tensor.max().item()

    # Calculate scale
    scale = (max_val - min_val) / 255.0

    # Quantize
    quantized = torch.round((tensor - min_val) / scale).clamp(0, 255).to(torch.uint8)

    return quantized, scale


def save_partition(
    partition_state: Dict[str, torch.Tensor],
    output_path: Path,
    quantization: str,
):
    """
    Save a partition to Safetensors format.

    Args:
        partition_state: Partition state dict
        output_path: Output file path
        quantization: Quantization type ("none", "int8", "int4")
    """
    if quantization == "int8":
        # Quantize all float tensors
        quantized_state = {}
        scales = {}

        for key, value in partition_state.items():
            if value.dtype == torch.float16 or value.dtype == torch.float32:
                quantized, scale = quantize_int8(value)
                quantized_state[key] = quantized
                scales[key] = scale
            else:
                quantized_state[key] = value

        # Save quantized tensors
        save_file(quantized_state, output_path)

        # Save quantization metadata
        metadata_path = output_path.with_suffix(".json")
        with open(metadata_path, "w") as f:
            json.dump({"scales": scales, "quantization": "int8"}, f, indent=2)

    else:
        # Save without quantization
        save_file(partition_state, output_path)


def generate_partition_metadata(
    model,
    layer_groups: List[Tuple[int, int]],
    output_dir: Path,
    quantization: str,
) -> Dict:
    """
    Generate metadata for all partitions.

    Args:
        model: The model
        layer_groups: List of layer group tuples
        output_dir: Output directory
        quantization: Quantization type

    Returns:
        Metadata dictionary
    """
    # Get model config
    if hasattr(model, "config"):
        config = model.config
        hidden_dim = getattr(config, "hidden_size", 4096)
        num_heads = getattr(config, "num_attention_heads", 32)
        head_dim = hidden_dim // num_heads
        intermediate_dim = getattr(config, "intermediate_size", 11008)
        vocab_size = getattr(config, "vocab_size", 32000)
    else:
        # Default values
        hidden_dim = 4096
        num_heads = 32
        head_dim = 128
        intermediate_dim = 11008
        vocab_size = 32000

    metadata = {
        "model_name": model.config._name_or_path if hasattr(model, "config") else "unknown",
        "num_partitions": len(layer_groups),
        "quantization": quantization,
        "partitions": [],
        "model_config": {
            "hidden_dim": hidden_dim,
            "num_heads": num_heads,
            "head_dim": head_dim,
            "intermediate_dim": intermediate_dim,
            "vocab_size": vocab_size,
        },
    }

    # Add metadata for each partition
    for i, (start, end) in enumerate(layer_groups):
        partition_info = {
            "partition_id": i,
            "start_layer": start,
            "end_layer": end,
            "num_layers": end - start,
            "file_path": f"partition_{i}.safetensors",
        }
        metadata["partitions"].append(partition_info)

    # Save metadata
    metadata_path = output_dir / "partition_metadata.json"
    with open(metadata_path, "w") as f:
        json.dump(metadata, f, indent=2)

    print(f"Saved partition metadata to {metadata_path}")

    return metadata


def main():
    args = parse_args()

    print(f"Loading model: {args.model}")
    print(f"Output directory: {args.output}")
    print(f"Number of partitions: {args.num_partitions}")
    print(f"Quantization: {args.quantization}")

    # Create output directory
    output_dir = Path(args.output)
    output_dir.mkdir(parents=True, exist_ok=True)

    # Load model
    print("Loading model...")
    model = AutoModelForCausalLM.from_pretrained(
        args.model,
        torch_dtype=torch.float16,
        device_map="cpu",
        trust_remote_code=True,
    )

    # Load tokenizer
    print("Loading tokenizer...")
    tokenizer = AutoTokenizer.from_pretrained(args.model, trust_remote_code=True)

    # Save tokenizer
    tokenizer_output = args.tokenizer_output or str(output_dir / "tokenizer")
    tokenizer.save_pretrained(tokenizer_output)
    print(f"Saved tokenizer to {tokenizer_output}")

    # Get state dict
    print("Extracting state dict...")
    state_dict = model.state_dict()

    # Calculate layer groups
    layer_groups = get_layer_groups(model, args.num_partitions)

    # Split and save partitions
    print("Splitting model into partitions...")
    for i, layer_group in enumerate(layer_groups):
        print(f"Processing partition {i}: layers {layer_group[0]}-{layer_group[1]}")

        # Extract partition
        partition_state = split_partition(
            state_dict,
            layer_group,
            i,
            model,
        )

        # Save partition
        output_path = output_dir / f"partition_{i}.safetensors"
        save_partition(partition_state, output_path, args.quantization)

        print(f"  Saved partition {i} to {output_path}")
        print(f"  Partition size: {output_path.stat().st_size / (1024**3):.2f} GB")

    # Generate metadata
    print("Generating partition metadata...")
    metadata = generate_partition_metadata(model, layer_groups, output_dir, args.quantization)

    print("\nModel splitting complete!")
    print(f"Output directory: {output_dir}")
    print(f"Number of partitions: {len(layer_groups)}")

    if args.quantization != "none":
        print("Note: Quantization is applied. Make sure workers support the quantization format.")


if __name__ == "__main__":
    main()
