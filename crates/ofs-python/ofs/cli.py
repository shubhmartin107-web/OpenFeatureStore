"""OpenFeatureStore CLI - manage feature stores from the command line."""

import argparse
import json
import sys
from typing import Dict, List, Optional

from .feature_store import FeatureStore


def cmd_init(args: argparse.Namespace) -> None:
    """Initialize a new feature store."""
    store = FeatureStore.in_memory(project=args.project or "default")
    print(f"Feature store initialized (project={store._project})")


def cmd_apply_entity(args: argparse.Namespace) -> None:
    """Register an entity."""
    store = _get_store(args)
    join_keys = args.join_keys.split(",") if args.join_keys else []
    store.apply_entity(args.name, join_keys)
    print(f"Entity '{args.name}' applied")


def cmd_apply_feature_view(args: argparse.Namespace) -> None:
    """Register a feature view."""
    store = _get_store(args)
    entities = args.entities.split(",") if args.entities else []
    features_list = args.features.split(",") if args.features else []
    store.apply_feature_view(args.name, entities, features_list, args.ttl)
    print(f"FeatureView '{args.name}' applied")


def cmd_list_entities(args: argparse.Namespace) -> None:
    """List all entities."""
    store = _get_store(args)
    entities = store.list_entities()
    if not entities:
        print("No entities found")
        return
    for e in entities:
        print(f"  {e['name']}: join_keys={e['join_keys']}")


def cmd_list_feature_views(args: argparse.Namespace) -> None:
    """List all feature views."""
    store = _get_store(args)
    fvs = store.list_feature_views()
    if not fvs:
        print("No feature views found")
        return
    for fv in fvs:
        print(f"  {fv['name']}: entities={fv['entities']}, features={fv['features']}")


def cmd_materialize(args: argparse.Namespace) -> None:
    """Materialize features from offline to online store."""
    store = _get_store(args)
    store.materialize(args.start_date, args.end_date)
    print(f"Materialized from {args.start_date} to {args.end_date}")


def cmd_incremental(args: argparse.Namespace) -> None:
    """Incremental materialization."""
    store = _get_store(args)
    store.materialize_incremental(args.end_date)
    print(f"Incremental materialization up to {args.end_date}")


def _get_store(args: argparse.Namespace) -> FeatureStore:
    return FeatureStore.in_memory(project=args.project or "default")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="OpenFeatureStore - Feature Store CLI",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  ofs init --project my_project
  ofs apply-entity user --join-keys user_id
  ofs apply-feature-view user_features --entities user --features age,gender --ttl 86400
  ofs materialize --start-date 1700000000 --end-date 1700086400
  ofs list-entities
  ofs list-feature-views
        """,
    )
    parser.add_argument(
        "--project", "-p", default=None, help="Project name (default: 'default')"
    )
    parser.add_argument(
        "--store", "-s", default="memory", choices=["memory"],
        help="Store backend (default: memory)"
    )

    subparsers = parser.add_subparsers(dest="command", help="Command to execute")

    # init
    p_init = subparsers.add_parser("init", help="Initialize a feature store")

    # apply-entity
    p_entity = subparsers.add_parser("apply-entity", help="Register an entity")
    p_entity.add_argument("name", help="Entity name")
    p_entity.add_argument("--join-keys", help="Comma-separated join key names")

    # apply-feature-view
    p_fv = subparsers.add_parser("apply-feature-view", help="Register a feature view")
    p_fv.add_argument("name", help="Feature view name")
    p_fv.add_argument("--entities", help="Comma-separated entity names")
    p_fv.add_argument("--features", help="Comma-separated feature names")
    p_fv.add_argument("--ttl", type=int, default=None, help="TTL in seconds")

    # list-entities
    subparsers.add_parser("list-entities", help="List all entities")

    # list-feature-views
    subparsers.add_parser("list-feature-views", help="List all feature views")

    # materialize
    p_mat = subparsers.add_parser("materialize", help="Materialize features")
    p_mat.add_argument("--start-date", type=float, required=True, help="Start Unix timestamp")
    p_mat.add_argument("--end-date", type=float, required=True, help="End Unix timestamp")

    # incremental
    p_inc = subparsers.add_parser("incremental", help="Incremental materialization")
    p_inc.add_argument("--end-date", type=float, required=True, help="End Unix timestamp")

    args = parser.parse_args()
    if args.command is None:
        parser.print_help()
        sys.exit(1)

    command_map = {
        "init": cmd_init,
        "apply-entity": cmd_apply_entity,
        "apply-feature-view": cmd_apply_feature_view,
        "list-entities": cmd_list_entities,
        "list-feature-views": cmd_list_feature_views,
        "materialize": cmd_materialize,
        "incremental": cmd_incremental,
    }

    cmd = command_map.get(args.command)
    if cmd:
        cmd(args)
    else:
        parser.print_help()
        sys.exit(1)


if __name__ == "__main__":
    main()
