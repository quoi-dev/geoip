import React, { type PropsWithChildren, type RefObject, useCallback, useEffect, useRef } from "react";
import { createPortal } from "react-dom";
import classNames from "classnames";

export interface DialogProps extends PropsWithChildren {
	title?: string;
	shown?: boolean;
	onClose?: () => void;
	primaryButton?: string;
	primaryButtonClassName?: string | null;
	onPrimaryButtonClick?: () => void;
	cancelButton?: string;
	formRef?: RefObject<HTMLFormElement | null>;
}

export const Dialog: React.FC<DialogProps> = ({
	title,
	shown,
	onClose,
	primaryButton,
	primaryButtonClassName,
	onPrimaryButtonClick,
	cancelButton,
	formRef,
	children,
}) => {
	const ref = useRef<HTMLDialogElement>(null);
	
	useEffect(() => {
		if (!ref.current) return;
		if (shown) {
			if (!ref.current.open) {
				ref.current?.showModal();
			}
		} else {
			if (ref.current.open) {
				ref.current.close();
			}
		}
	}, [shown]);
	
	const handlePrimaryButtonClick = useCallback(() => {
		formRef?.current?.requestSubmit();
		onPrimaryButtonClick?.();
	}, [onPrimaryButtonClick, formRef]);
	
	const handleClose = useCallback(() => {
		onClose?.();
	}, [onClose]);
	
	return createPortal(
		(
			<dialog ref={ref} className="modal" onClose={handleClose}>
				<div className="modal-box">
					{
						title !== undefined && (
							<h3 className="text-lg font-bold mb-4">{title}</h3>
						)
					}
					{children}
					{
						(primaryButton !== undefined || cancelButton !== undefined) && (
							<div className="modal-action">
								{
									primaryButton !== undefined && (
										<button
											className={classNames("btn", primaryButtonClassName ?? "btn-neutral")}
											onClick={handlePrimaryButtonClick}
										>
											{primaryButton}
										</button>
									)
								}
								{
									cancelButton !== undefined && (
										<form method="dialog">
											<button className="btn">
												{cancelButton}
											</button>
										</form>
									)
								}
							</div>
						)
					}
				</div>
			</dialog>
		),
		document.body,
	);
};
